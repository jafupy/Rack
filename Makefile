SHELL := /bin/zsh

ROOT_DIR := $(CURDIR)
BUILD_DIR := $(ROOT_DIR)/.build
DIST_DIR := $(ROOT_DIR)/dist
RELEASE_DIR := $(BUILD_DIR)/release
APP_DIR := $(DIST_DIR)/Rack.app
MACOS_DIR := $(APP_DIR)/Contents/MacOS
RESOURCES_DIR := $(APP_DIR)/Contents/Resources
PLIST_PATH := $(APP_DIR)/Contents/Info.plist
DMG_ROOT := $(DIST_DIR)/dmg-root
DMG_PATH := $(DIST_DIR)/Rack.dmg
APP_LINK := /Applications/Rack.app
SRC_DIR := $(ROOT_DIR)/Sources

UNAME_M := $(shell uname -m)
ifeq ($(UNAME_M),arm64)
RUST_TARGET := aarch64-apple-darwin
else
RUST_TARGET := x86_64-apple-darwin
endif

RACK_BRIDGE_BIN := $(BUILD_DIR)/rust/$(RUST_TARGET)/release/rack-bridge
RACK_CLI_BIN := $(BUILD_DIR)/rust/$(RUST_TARGET)/release/rack

.PHONY: help run app dmg shim-app dev clean

help:
	@printf "Targets:\n"
	@printf "  make run       Run Rack with SwiftPM\n"
	@printf "  make app       Build dist/Rack.app\n"
	@printf "  make dmg       Build dist/Rack.dmg\n"
	@printf "  make shim-app  Point /Applications/Rack.app at dist/Rack.app\n"
	@printf "  make dev       Watch Sources, rebuild, and relaunch Rack.app\n"
	@printf "  make clean     Remove build artifacts\n"

run:
	@swift run

app:
	@set -euo pipefail; \
	mkdir -p "$(BUILD_DIR)" "$(DIST_DIR)"; \
	cargo build --release \
		--manifest-path "$(ROOT_DIR)/Cargo.toml" \
		--target-dir "$(BUILD_DIR)/rust" \
		--target "$(RUST_TARGET)"; \
	swift build --configuration release --product Rack --scratch-path "$(BUILD_DIR)"; \
	executable_path=""; \
	if [[ -x "$(BUILD_DIR)/arm64-apple-macosx/release/Rack" ]]; then \
		executable_path="$(BUILD_DIR)/arm64-apple-macosx/release/Rack"; \
	elif [[ -x "$(BUILD_DIR)/x86_64-apple-macosx/release/Rack" ]]; then \
		executable_path="$(BUILD_DIR)/x86_64-apple-macosx/release/Rack"; \
	elif [[ -x "$(RELEASE_DIR)/Rack" ]]; then \
		executable_path="$(RELEASE_DIR)/Rack"; \
	else \
		echo "Could not find release executable." >&2; \
		exit 1; \
	fi; \
	rm -rf "$(APP_DIR)"; \
	mkdir -p "$(MACOS_DIR)" "$(RESOURCES_DIR)"; \
	cp "$$executable_path" "$(MACOS_DIR)/Rack"; \
	chmod +x "$(MACOS_DIR)/Rack"; \
	cp "$(RACK_BRIDGE_BIN)" "$(RESOURCES_DIR)/rack-bridge"; \
	chmod +x "$(RESOURCES_DIR)/rack-bridge"; \
	cp "$(RACK_CLI_BIN)" "$(RESOURCES_DIR)/rack"; \
	chmod +x "$(RESOURCES_DIR)/rack"; \
	printf '%s\n' \
		'<?xml version="1.0" encoding="UTF-8"?>' \
		'<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">' \
		'<plist version="1.0">' \
		'<dict>' \
		'  <key>CFBundleDevelopmentRegion</key>' \
		'  <string>en</string>' \
		'  <key>CFBundleExecutable</key>' \
		'  <string>Rack</string>' \
		'  <key>CFBundleIdentifier</key>' \
		'  <string>com.jafupy.Rack</string>' \
		'  <key>CFBundleInfoDictionaryVersion</key>' \
		'  <string>6.0</string>' \
		'  <key>CFBundleName</key>' \
		'  <string>Rack.</string>' \
		'  <key>CFBundleDisplayName</key>' \
		'  <string>Rack.</string>' \
		'  <key>CFBundlePackageType</key>' \
		'  <string>APPL</string>' \
		'  <key>CFBundleURLTypes</key>' \
		'  <array>' \
		'    <dict>' \
		'      <key>CFBundleURLName</key>' \
		'      <string>Rack URL</string>' \
		'      <key>CFBundleURLSchemes</key>' \
		'      <array>' \
		'        <string>rack</string>' \
		'      </array>' \
		'    </dict>' \
		'  </array>' \
		'  <key>CFBundleShortVersionString</key>' \
		'  <string>0.1.0</string>' \
		'  <key>CFBundleVersion</key>' \
		'  <string>1</string>' \
		'  <key>LSApplicationCategoryType</key>' \
		'  <string>public.app-category.developer-tools</string>' \
		'  <key>LSMinimumSystemVersion</key>' \
		'  <string>14.0</string>' \
		'  <key>LSUIElement</key>' \
		'  <true/>' \
		'  <key>NSHighResolutionCapable</key>' \
		'  <true/>' \
		'</dict>' \
		'</plist>' \
		> "$(PLIST_PATH)"; \
	echo "Built app bundle at $(APP_DIR)"

dmg: app
	@set -euo pipefail; \
	rm -rf "$(DMG_ROOT)" "$(DMG_PATH)"; \
	mkdir -p "$(DMG_ROOT)"; \
	cp -R "$(APP_DIR)" "$(DMG_ROOT)/"; \
	ln -s /Applications "$(DMG_ROOT)/Applications"; \
	hdiutil create \
		-volname "Rack" \
		-srcfolder "$(DMG_ROOT)" \
		-ov \
		-format UDZO \
		"$(DMG_PATH)"; \
	rm -rf "$(DMG_ROOT)"; \
	echo "Built DMG at $(DMG_PATH)"

shim-app:
	@set -euo pipefail; \
	if [[ ! -d "$(APP_DIR)" ]]; then \
		echo "Missing $(APP_DIR). Run make app first." >&2; \
		exit 1; \
	fi; \
	if [[ -e "$(APP_LINK)" && ! -L "$(APP_LINK)" ]]; then \
		echo "$(APP_LINK) exists and is not a symlink. Move it aside before shimming." >&2; \
		exit 1; \
	fi; \
	ln -sfn "$(APP_DIR)" "$(APP_LINK)"; \
	echo "$(APP_LINK) -> $(APP_DIR)"

dev:
	@set -euo pipefail; \
	snapshot() { \
		find "$(SRC_DIR)" -type f -print0 | xargs -0 stat -f "%m %N" | sort | shasum; \
	}; \
	relaunch() { \
		pkill -f "Rack.app/Contents/MacOS/Rack" 2>/dev/null || true; \
		open "$(APP_DIR)" || true; \
	}; \
	build_and_launch() { \
		echo; \
		echo "[build] Building..."; \
		start=$$(date +%s); \
		if $(MAKE) app; then \
			elapsed=$$(( $$(date +%s) - $$start )); \
			echo; \
			echo "[build] Built in $${elapsed}s; relaunching"; \
			relaunch; \
		else \
			echo; \
			echo "[build] Build failed" >&2; \
		fi; \
	}; \
	echo "Watching $(SRC_DIR)"; \
	last="$$(snapshot)"; \
	build_and_launch; \
	while true; do \
		sleep 1; \
		current="$$(snapshot)"; \
		if [[ "$$current" != "$$last" ]]; then \
			last="$$current"; \
			build_and_launch; \
		fi; \
	done

clean:
	@rm -rf "$(BUILD_DIR)" "$(DIST_DIR)"
