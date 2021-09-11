#![no_main]
#![no_std]

use panic_halt as _;

use core::convert::Infallible;
use hal::prelude::*;
use hal::gpio::{Input, Output, Pxx, PullUp, PushPull, OpenDrain};
use hal::serial;
use hal::usb;
use hal::{stm32, timer};
use embedded_hal::digital::v2::OutputPin;
use keyberon::action::{k, l, m, d, Action, Action::*, HoldTapConfig};
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::key_code::KeyCode::*;
use keyberon::layout::{Event, Layout};
use keyberon::matrix::{Matrix, PressedKeys};
use nb::block;
use rtic::app;
use stm32f1xx_hal as hal;
use usb_device::bus::UsbBusAllocator;
use usb_device::class::UsbClass as _;
use usb_device::device::UsbDeviceState;

type UsbClass = keyberon::Class<'static, usb::UsbBusType, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, usb::UsbBusType>;

trait ResultExt<T> {
    fn get(self) -> T;
}
impl<T> ResultExt<T> for Result<T, Infallible> {
    fn get(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => match e {},
        }
    }
}

// Taps will toggle the default layer, holding will add acurrent layer.
const FN_KEY: Action = HoldTap {
    config: HoldTapConfig::Default,
    hold: &l(1),
    tap: &d(1),
    tap_hold_interval: 0,
    timeout: 100,
};

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers = &[
    &[
        &[k(Grave), k(Kb1), k(Kb2), k(Kb3), k(Kb4), k(Kb5), k(Kb6), Trans, Trans, k(Kb6), k(Kb7), k(Kb8), k(Kb9), k(Kb0), k(Minus), k(Equal), Trans, k(BSpace)],
        &[k(Tab), Trans, k(Q), k(W), k(E), k(R), k(T), k(Y), Trans, Trans, k(Y), k(U), k(I), k(O), k(P), k(LBracket), k(RBracket), Trans],
        &[k(Escape), Trans, k(A), k(S), k(D), k(F), k(G), k(H), Trans, k(BSpace), k(H), k(J), k(K), k(L), k(SColon), k(Quote), k(NonUsHash), k(Enter)],
        &[k(LShift), k(NonUsBslash), k(Z), k(X), k(C), k(V), k(B), Trans, Trans, Trans, k(B), k(N), k(M), k(Comma), k(Dot), k(Slash), k(Up), k(RShift)],
        &[k(LCtrl), k(LGui), Trans, k(LAlt), FN_KEY, k(Space), Trans, Trans, Trans, Trans, k(Space), Trans, k(Enter), k(Delete), k(RAlt), k(Left), k(Down), k(Right)]
    ],
    &[
        &[Trans, k(F1), k(F2), k(F3), k(F4), k(F5), k(F6), Trans, Trans, k(F6), k(F7), k(F8), k(F9), k(F10), k(F11), k(F12), Trans, Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, k(Kp7), k(Kp8), k(Kp9), Trans, Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, k(Kp4), k(Kp5), k(Kp6), Trans, Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, k(Kp1), k(Kp2), k(Kp3), Trans, Trans],
        &[Trans, Trans, Trans, Trans, d(0), Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, k(Kp0), Trans, Trans, Trans],
    ],
];

#[derive(Debug, Clone, Copy)]
pub enum BoardType {
    Left,
    Right
}

const COLS_PER_BOARD: u8 = 9;

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        matrix: Matrix<Pxx<Input<PullUp>>, Pxx<Output<PushPull>>, 9, 5>,
        debouncer: Debouncer<PressedKeys<9, 5>>,
        layout: Layout,
        timer: timer::CountDownTimer<stm32::TIM3>,

        right_tx: serial::Tx<hal::pac::USART1>,
        right_rx: serial::Rx<hal::pac::USART1>,

        left_tx: serial::Tx<hal::pac::USART3>,
        left_rx: serial::Rx<hal::pac::USART3>,

        left_buf: [u8; 4],
        right_buf: [u8; 4],

        board_type: BoardType, 

        leds: [Pxx<Output<OpenDrain>>; 3]
        /*
        transform: fn(Event) -> Event,
        tx: serial::Tx<hal::pac::USART1>,
        rx: serial::Rx<hal::pac::USART1>,
        */
    }

    #[init]
    fn init(mut c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        let mut flash = c.device.FLASH.constrain();
        let mut rcc = c.device.RCC.constrain();

        let mut afio = c.device.AFIO.constrain(&mut rcc.apb2);

        let clocks = rcc.cfgr
            .use_hse(8.mhz())
            .sysclk(72.mhz())
            .pclk1(36.mhz())
            .pclk2(72.mhz())
            .freeze(&mut flash.acr);

        let mut gpioa = c.device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = c.device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = c.device.GPIOC.split(&mut rcc.apb2);

        let usb = usb::Peripheral {
            usb: c.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: gpioa.pa12,
        };
        *USB_BUS = Some(usb::UsbBusType::new(usb));
        let usb_bus = USB_BUS.as_ref().unwrap();

        let usb_class = keyberon::new_class(usb_bus, ());
        let usb_dev = keyberon::new_device(usb_bus);

        let mut timer = timer::Timer::tim3(c.device.TIM3, &clocks, &mut rcc.apb1).start_count_down(1.khz());
        timer.listen(timer::Event::Update);

        let mut right_usart = serial::Serial::usart1(c.device.USART1, 
            (
                gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl),
                gpiob.pb7
            ),
            &mut afio.mapr,
            serial::Config::default().baudrate(38_400.bps()).parity_even(),
            clocks,
            &mut rcc.apb2
        );
        right_usart.listen(serial::Event::Rxne);
        let (right_tx, right_rx) = right_usart.split();

        let mut left_usart = serial::Serial::usart3(c.device.USART3, 
            (
                gpiob.pb10.into_alternate_push_pull(&mut gpiob.crh),
                gpiob.pb11
            ),
            &mut afio.mapr,
            serial::Config::default().baudrate(38_400.bps()).parity_even(),
            clocks,
            &mut rcc.apb1
        );
        left_usart.listen(serial::Event::Rxne);
        let (left_tx, left_rx) = left_usart.split();

        /*
        let (pa9, pa10) = (gpioa.pa9, gpioa.pa10);
        let pins = cortex_m::interrupt::free(move |cs| {
            (pa9.into_alternate_af1(cs), pa10.into_alternate_af1(cs))
        });
        let mut serial = serial::Serial::usart1(c.device.USART1, pins, 38_400.bps(), &mut rcc);
        serial.listen(serial::Event::Rxne);
        let (tx, rx) = serial.split();
        */

        let mut matrix = Matrix::new(
            [
                gpioa.pa8.into_pull_up_input(&mut gpioa.crh).downgrade(),
                gpioa.pa9.into_pull_up_input(&mut gpioa.crh).downgrade(),
                gpioa.pa10.into_pull_up_input(&mut gpioa.crh).downgrade(),
                gpioa.pa6.into_pull_up_input(&mut gpioa.crl).downgrade(),
                gpioa.pa7.into_pull_up_input(&mut gpioa.crl).downgrade(),
                gpiob.pb0.into_pull_up_input(&mut gpiob.crl).downgrade(),
                gpiob.pb12.into_pull_up_input(&mut gpiob.crh).downgrade(),
                gpiob.pb13.into_pull_up_input(&mut gpiob.crh).downgrade(),
                gpiob.pb14.into_pull_up_input(&mut gpiob.crh).downgrade(),
            ],
            [

                gpiob.pb5.into_push_pull_output(&mut gpiob.crl).downgrade(),
                gpiob.pb8.into_push_pull_output(&mut gpiob.crh).downgrade(),
                gpioa.pa0.into_push_pull_output(&mut gpioa.crl).downgrade(),
                gpioc.pc15.into_push_pull_output(&mut gpioc.crh).downgrade(),
                gpioc.pc14.into_push_pull_output(&mut gpioc.crh).downgrade(),
            ],
        ).get();

        /* Bootloader decision, unfortunately we have to scan the full matrix here since Keyberon
         * has no better API right now */
        if matrix.get().unwrap().0[0][0] {
            let bkp = rcc
                .bkp
                .constrain(c.device.BKP, &mut rcc.apb1, &mut c.device.PWR);

            reboot_bootloader(bkp);
        }

        let leds = [
            gpioa.pa1.into_open_drain_output(&mut gpioa.crl).downgrade(),
            gpioa.pa2.into_open_drain_output(&mut gpioa.crl).downgrade(),
            gpioa.pa3.into_open_drain_output(&mut gpioa.crl).downgrade(),
        ];

        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
            matrix,
            layout: Layout::new(LAYERS),
            board_type: BoardType::Right,

            left_tx,
            left_rx,
            left_buf: [0; 4],

            right_tx,
            right_rx,
            right_buf: [0; 4],

            leds
        }
    }

    #[task(binds = USB_HP_CAN_TX, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_tx(c: usb_tx::Context) {
        usb_poll(c.resources.usb_dev, c.resources.usb_class);
    }

    #[task(binds = USB_LP_CAN_RX0, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        usb_poll(c.resources.usb_dev, c.resources.usb_class);
    }

    // TODO: Pipe from right to left side and vice versa
    #[task(binds = USART1, priority = 5, spawn = [handle_event], resources = [right_rx, right_buf])]
    fn uart_right_rx(c: uart_right_rx::Context) {
        if let Some(event) = read_packet(c.resources.right_rx, c.resources.right_buf) {
            c.spawn.handle_event(Some(event)).unwrap();
        }
    }

    #[task(binds = USART3, priority = 5, spawn = [handle_event], resources = [left_rx, left_buf])]
    fn uart_left_rx(c: uart_left_rx::Context) {
        if let Some(event) = read_packet(c.resources.left_rx, c.resources.left_buf) {
            c.spawn.handle_event(Some(event)).unwrap();
        }
    }
    
    #[task(priority = 3, capacity = 8, resources = [usb_dev, usb_class, layout, leds])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        match event {
            None => c.resources.layout.tick(),
            Some(e) => {
                c.resources.layout.event(e);
                return;
            }
        };

        /* Update LED states */
        let mut layer = c.resources.layout.current_layer();
        for led in c.resources.leds {
            if layer & 1 != 0 {
                led.set_low().unwrap();
            } else {
                led.set_high().unwrap();
            }
            layer >>= 1;
        }

        let report: KbHidReport = c.resources.layout.keycodes().collect();
        if !c
            .resources
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.resources.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.resources.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(
        binds = TIM3,
        priority = 2,
        resources = [usb_dev, usb_class, matrix, debouncer, timer, layout, right_tx, left_tx, board_type],
        spawn = [handle_event]
    )]
    fn tick(c: tick::Context) {
        c.resources.timer.clear_update_interrupt_flag();

        let bt = *c.resources.board_type;
        for event in c
            .resources
            .debouncer
            .events(c.resources.matrix.get().get())
            .map(|e| translate(bt, e))
        {
            for &b in &ser(event) {
                block!(c.resources.right_tx.write(b)).get();
                block!(c.resources.left_tx.write(b)).get();
            }
            c.spawn.handle_event(Some(event)).unwrap();
        }
        c.spawn.handle_event(None).unwrap();
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn EXTI0();
    }
};

fn usb_poll(usb_dev: &mut UsbDevice, keyboard: &mut UsbClass) {
    if usb_dev.poll(&mut [keyboard]) {
        keyboard.poll();
    }
}

fn de(bytes: &[u8]) -> Result<Event, ()> {
    match *bytes {
        [b'P', i, j, b'\n'] => Ok(Event::Press(i, j)),
        [b'R', i, j, b'\n'] => Ok(Event::Release(i, j)),
        _ => Err(()),
    }
}
fn ser(e: Event) -> [u8; 4] {
    match e {
        Event::Press(i, j) => [b'P', i, j, b'\n'],
        Event::Release(i, j) => [b'R', i, j, b'\n'],
    }
}

fn read_packet(rx: &mut impl embedded_hal::serial::Read<u8>, buffer: &mut [u8; 4]) -> Option<Event> {
    if let Ok(b) = rx.read() {
        buffer.rotate_left(1);
        buffer[3] = b;

        if buffer[3] == b'\n' {
            if let Ok(event) = de(&buffer[..]) {
                Some(event)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
    
}

fn translate(board_type: BoardType, event: Event) -> Event {
    match board_type {
        BoardType::Left => event,
        BoardType::Right => event.transform(|i,j| (i, COLS_PER_BOARD + j))

    }
}

/* This routine is copied from the great polymer firmware: https://gitlab.com/polymer-kb/firmware/polymer/-/blob/master/src/reboot.rs */
pub fn reboot_bootloader(_: stm32f1xx_hal::backup_domain::BackupDomain) -> ! {
    unsafe {
        // Note: Using raw offsets here because it's wrong in the upstream stm32f1 crate.
        const BKP_BASE: usize = 0x4000_6C00;
        const BKP_DR10_OFF: usize = 0x28;
        core::ptr::write_volatile((BKP_BASE + BKP_DR10_OFF) as *mut _, 0x424C)
    };
    reboot();
}

pub fn reboot() -> ! {
    stm32f1xx_hal::stm32::SCB::sys_reset()
}
