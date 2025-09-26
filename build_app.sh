#!/bin/bash

# Build script for creating macOS app bundle for Rustwave

set -e

echo "Building Rustwave macOS app bundle..."

# Clean previous build
rm -rf Rustwave.app

# Build the release binary
echo "Building release binary..."
cargo build --release

# Create app bundle structure
echo "Creating app bundle structure..."
mkdir -p Rustwave.app/Contents/MacOS
mkdir -p Rustwave.app/Contents/Resources

# Copy the executable
echo "Copying executable..."
cp target/release/rustwave Rustwave.app/Contents/MacOS/

# Create Info.plist
echo "Creating Info.plist..."
cat > Rustwave.app/Contents/Info.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>rustwave</string>
    <key>CFBundleIdentifier</key>
    <string>com.rustwave.app</string>
    <key>CFBundleName</key>
    <string>Rustwave</string>
    <key>CFBundleDisplayName</key>
    <string>Rustwave</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

# Create iconset if it doesn't exist
if [ ! -d "AppIcon.iconset" ]; then
    echo "Creating icon set..."
    mkdir -p AppIcon.iconset
    
    # Generate different icon sizes
    sips -z 16 16 assets/icon.png --out AppIcon.iconset/icon_16x16.png
    sips -z 32 32 assets/icon.png --out AppIcon.iconset/icon_16x16@2x.png
    sips -z 32 32 assets/icon.png --out AppIcon.iconset/icon_32x32.png
    sips -z 64 64 assets/icon.png --out AppIcon.iconset/icon_32x32@2x.png
    sips -z 128 128 assets/icon.png --out AppIcon.iconset/icon_128x128.png
    sips -z 256 256 assets/icon.png --out AppIcon.iconset/icon_128x128@2x.png
    sips -z 256 256 assets/icon.png --out AppIcon.iconset/icon_256x256.png
    sips -z 512 512 assets/icon.png --out AppIcon.iconset/icon_256x256@2x.png
    sips -z 512 512 assets/icon.png --out AppIcon.iconset/icon_512x512.png
    cp assets/icon.png AppIcon.iconset/icon_512x512@2x.png
fi

# Convert iconset to icns
echo "Converting iconset to icns..."
iconutil -c icns AppIcon.iconset

# Copy icon to app bundle
echo "Copying icon to app bundle..."
cp AppIcon.icns Rustwave.app/Contents/Resources/

# Make executable
chmod +x Rustwave.app/Contents/MacOS/rustwave

echo "App bundle created successfully!"
echo "You can now run: open Rustwave.app"
echo "Or double-click Rustwave.app in Finder"
