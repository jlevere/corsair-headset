APP_NAME := Corsair Headset
BUNDLE_ID := com.jlevere.corsair-headset
VERSION := 0.1.0

APP_DIR := target/release/$(APP_NAME).app
DMG_NAME := Corsair-Headset-$(VERSION).dmg

.PHONY: app dmg clean release

# Build the release binary
target/release/corsair-tray: $(shell find crates -name '*.rs') Cargo.toml
	cargo build --release -p corsair-tray

# Assemble the .app bundle
app: target/release/corsair-tray
	@echo "Assembling $(APP_NAME).app..."
	@rm -rf "$(APP_DIR)"
	@mkdir -p "$(APP_DIR)/Contents/MacOS"
	@mkdir -p "$(APP_DIR)/Contents/Resources"
	@cp bundle/Info.plist "$(APP_DIR)/Contents/"
	@cp target/release/corsair-tray "$(APP_DIR)/Contents/MacOS/"
	@cp bundle/AppIcon.icns "$(APP_DIR)/Contents/Resources/"
	@echo "APPL????" > "$(APP_DIR)/Contents/PkgInfo"
	@echo "Built: $(APP_DIR)"

# Create a .dmg disk image with drag-to-Applications
dmg: app
	@echo "Creating $(DMG_NAME)..."
	@rm -rf /tmp/dmg-stage
	@mkdir -p /tmp/dmg-stage
	@cp -R "$(APP_DIR)" /tmp/dmg-stage/
	@ln -s /Applications /tmp/dmg-stage/Applications
	@hdiutil create -volname "Corsair Headset" \
		-srcfolder /tmp/dmg-stage \
		-ov -format UDZO \
		"target/release/$(DMG_NAME)" 2>/dev/null
	@rm -rf /tmp/dmg-stage
	@echo "Built: target/release/$(DMG_NAME)"

# Build everything for release
release: dmg
	@echo ""
	@echo "Release artifacts:"
	@ls -lh "target/release/$(DMG_NAME)"
	@ls -lh "$(APP_DIR)/Contents/MacOS/corsair-tray"

clean:
	cargo clean
	rm -rf "target/release/$(APP_NAME).app"
