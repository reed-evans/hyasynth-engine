#! /bin/bash

# Build for all targets
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim

rm -rf Hyasynth.xcframework

# Create XCFramework
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libhyasynth.a \
    -headers include/ \
    -library target/aarch64-apple-ios-sim/release/libhyasynth.a \
    -headers include/ \
    -output Hyasynth.xcframework