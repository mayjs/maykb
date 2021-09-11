#!/usr/bin/env bash

set -euo pipefail

cd "${0%/*}"

cargo objcopy --bin maykb_gen2 --release -- -O binary build/maykb_gen2.bin

sudo dfu-util -a 2 -d 1209:9200 -D build/maykb_gen2.bin
