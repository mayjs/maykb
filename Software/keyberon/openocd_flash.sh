#!/bin/sh
OPENOCD="$(which openocd)"
OPENOCD_SCRIPTS="$(dirname $OPENOCD)/../share/openocd/scripts"
FWBIN="./build/maykb_gen2.bin"

cargo objcopy --bin maykb_gen2 --release -- -O binary $FWBIN

sudo openocd -c "set CPUTAPID 0x2ba01477" -f $OPENOCD_SCRIPTS/interface/stlink.cfg -f $OPENOCD_SCRIPTS/target/stm32f1x.cfg -c "program $FWBIN verify exit 0x08002000"
