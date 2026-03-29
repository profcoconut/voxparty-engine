# VoxParty iOS

## Build Rust Library

```bash
# Install iOS target (once)
rustup target add aarch64-apple-ios

# Build for iOS device (arm64)
cargo build --target aarch64-apple-ios --release

# Build for iOS simulator (x86_64 + arm64 macabi)
cargo build --target aarch64-apple-ios-macabi --release
```

The static library is at `target/aarch64-apple-ios/release/libvoxparty.a`.

## Xcode Project Setup

1. Create a new iOS Game project in Xcode
2. Add `libvoxparty.a` to "Link Binary With Libraries"
3. Add SDL2.framework (from Homebrew: `/opt/homebrew/lib/libSDL2.dylib`) or via CocoaPods (SDL2)
4. Create a bridging header that calls into the Rust library:

```objc
// VoxParty-Bridging-Header.h
#import "voxparty_voxparty.h"
```

5. In your AppDelegate or main.m, call:
```objc
voxparty_run();
```

## Dependencies (via Homebrew)

```bash
brew install sdl2 sdl2_image sdl2_mixer sdl2_ttf
```

Or use CocoaPods:
```ruby
pod 'SDL2', '~> 2.32'
```
