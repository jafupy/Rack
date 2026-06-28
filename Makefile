.PHONY: all rust swift app dist clean

APP_NAME := rack
PRODUCT_NAME := Rack
BUNDLE_ID := dev.jafu.rack
CONFIGURATION := debug
RUST_LIB := .build/rust/$(CONFIGURATION)/deps/librack_services.dylib
SWIFT_BIN := .build/arm64-apple-macosx/$(CONFIGURATION)/$(PRODUCT_NAME)
APP := dist/$(APP_NAME).app
CONTENTS := $(APP)/Contents
MACOS := $(CONTENTS)/MacOS
FRAMEWORKS := $(CONTENTS)/Frameworks
PLIST := $(CONTENTS)/Info.plist
PLISTBUDDY := /usr/libexec/PlistBuddy

all: app

dist: app

rust:
	cargo build -p rack-services

swift: rust
	swift build

app: swift
	rm -rf $(APP)
	mkdir -p $(MACOS) $(FRAMEWORKS)
	cp $(SWIFT_BIN) $(MACOS)/$(PRODUCT_NAME)
	cp $(RUST_LIB) $(FRAMEWORKS)/librack_services.dylib
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
