PORT ?= /dev/ttyUSB0
BINARY = target/xtensa-esp32-espidf/release/healing-tank-screen

# Sources ESP toolchain and appends the nix-ld lib dir to LD_LIBRARY_PATH so
# foreign binaries (esp-clang, ldproxy) can find libstdc++/libxml2 on NixOS.
# On non-NixOS systems the nix-ld path doesn't exist and is harmlessly ignored.
NIX_LD_LIBS = /run/current-system/sw/share/nix-ld/lib
# libxml2-compat provides libxml2.so.2 which esp-clang requires (system only has .so.16)
NIX_XML2_COMPAT = /nix/store/5kfg12jmldjbrm590n9znbcpxmf2pnpg-libxml2-compat/lib
ESP_ENV = source ~/export-esp.sh && export LD_LIBRARY_PATH="$(NIX_XML2_COMPAT):$(NIX_LD_LIBS)$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH}"

.PHONY: build check flash monitor flash-monitor clean

build:
	bash -c '$(ESP_ENV) && cargo build --release'

check:
	bash -c '$(ESP_ENV) && cargo check'

flash: build
	espflash flash $(BINARY) --port $(PORT)

monitor:
	espflash monitor --port $(PORT)

flash-monitor: flash
	espflash monitor --port $(PORT)

clean:
	cargo clean
