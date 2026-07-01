.PHONY: all rust swift app dist clean

APP_NAME := rack
PRODUCT_NAME := Rack
BUNDLE_ID := dev.jafu.rack
CONFIGURATION := debug
ifneq ($(filter $(CONFIGURATION),debug release),$(CONFIGURATION))
$(error CONFIGURATION must be debug or release)
endif
CARGO_PROFILE_DIR := $(CONFIGURATION)
CARGO_PROFILE_FLAG := $(if $(filter release,$(CARGO_PROFILE_DIR)),--release,)
SWIFT_CONFIGURATION := $(CONFIGURATION)
SWIFT_BIN_PATH := .build/rack-swift-bin-path
RUST_LIB := .build/rust/$(CARGO_PROFILE_DIR)/deps/librack_services.dylib
RUST_CLI := .build/rust/$(CARGO_PROFILE_DIR)/rack-cli
APP := dist/$(APP_NAME).app
CONTENTS := $(APP)/Contents
MACOS := $(CONTENTS)/MacOS
FRAMEWORKS := $(CONTENTS)/Frameworks
RESOURCES := $(CONTENTS)/Resources
PLIST := $(CONTENTS)/Info.plist
PLISTBUDDY := /usr/libexec/PlistBuddy

all: app

dist: app

rust:
	cargo build $(CARGO_PROFILE_FLAG) -p rack-services -p rack-cli

swift: rust
	RACK_RUST_PROFILE_DIR=$(CARGO_PROFILE_DIR) swift build --configuration $(SWIFT_CONFIGURATION)
	mkdir -p .build
	RACK_RUST_PROFILE_DIR=$(CARGO_PROFILE_DIR) swift build --configuration $(SWIFT_CONFIGURATION) --show-bin-path > $(SWIFT_BIN_PATH)

app: swift
	rm -rf $(APP)
	mkdir -p $(MACOS) $(FRAMEWORKS) $(RESOURCES)
	test -s $(SWIFT_BIN_PATH)
	cp "$$(cat $(SWIFT_BIN_PATH))/$(PRODUCT_NAME)" $(MACOS)/$(PRODUCT_NAME)
	cp $(RUST_LIB) $(FRAMEWORKS)/librack_services.dylib
	cp $(RUST_CLI) $(RESOURCES)/rack-cli
	install_name_tool -change $(abspath $(RUST_LIB)) @executable_path/../Frameworks/librack_services.dylib $(MACOS)/$(PRODUCT_NAME)
	printf '%s\n' '<?xml version="1.0" encoding="UTF-8"?>' '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">' '<plist version="1.0"><dict/></plist>' > $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleDevelopmentRegion string en' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleExecutable string $(PRODUCT_NAME)' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleIdentifier string $(BUNDLE_ID)' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleInfoDictionaryVersion string 6.0' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleName string $(PRODUCT_NAME)' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundlePackageType string APPL' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleShortVersionString string 1.0.0' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleVersion string 1' $(PLIST)
	$(PLISTBUDDY) -c 'Add :LSMinimumSystemVersion string 14.0' $(PLIST)
	$(PLISTBUDDY) -c 'Add :LSUIElement bool true' $(PLIST)
	$(PLISTBUDDY) -c 'Add :NSPrincipalClass string NSApplication' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleURLTypes array' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleURLTypes:0 dict' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleURLTypes:0:CFBundleURLName string $(BUNDLE_ID)' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleURLTypes:0:CFBundleURLSchemes array' $(PLIST)
	$(PLISTBUDDY) -c 'Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string rack' $(PLIST)
	plutil -lint $(PLIST)
	codesign --force --sign - $(APP)

clean:
	rm -rf dist
