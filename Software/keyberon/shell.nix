with import <nixpkgs> {};

let
rustChannel = rustChannelOf { channel="stable"; };
in
pkgs.mkShell {
  buildInputs = [
    (rustChannel.rust.override {
      targets = [ "thumbv7m-none-eabi" ];
    })
    rustChannel.rust-src
    cargo
    rust-analyzer

    # For flashing
    cargo-binutils
    dfu-util
    openocd
  ];

  shellHook = ''
    export RUST_SRC_PATH="${rustChannel.rust-src}/lib/rustlib/src/rust/library"
  '';
}
 
