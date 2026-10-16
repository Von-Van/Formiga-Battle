#!/usr/bin/env bash
# Builds Formiga Battle as a universal macOS app that Formiga Desktop can find: the bundle id and
# the battle version it reads are the ones formiga-battle-contract's discovery module names.
#
# The program inside the app is the file whose SHA-256 the friend-match relay knows a release by,
# so it is printed last, after signing, which changes it.
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$script_dir/.." && pwd)"
dist_dir="$repo_dir/dist"
app_dir="$dist_dir/Formiga Battle.app"
version="${FORMIGA_BATTLE_VERSION:-$(sed -n 's/^version = "\(.*\)"$/\1/p' "$repo_dir/Cargo.toml" | head -n 1)}"
version="${version#v}"
build_number="${FORMIGA_BATTLE_BUILD_NUMBER:-1}"
archive="$dist_dir/Formiga-Battle-$version-macOS-universal.zip"
disk_image="$dist_dir/Formiga-Battle-$version-macOS-universal.dmg"
dmg_staging="$dist_dir/Formiga-Battle-dmg"
program="$app_dir/Contents/MacOS/Formiga Battle"

cd "$repo_dir"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release -p formiga-battle --target aarch64-apple-darwin
cargo build --release -p formiga-battle --target x86_64-apple-darwin

rm -rf "$app_dir" "$dmg_staging"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
cp "$repo_dir/packaging/macos/Info.plist" "$app_dir/Contents/Info.plist"
lipo -create \
    "$repo_dir/target/aarch64-apple-darwin/release/formiga-battle" \
    "$repo_dir/target/x86_64-apple-darwin/release/formiga-battle" \
    -output "$program"
chmod 755 "$program"

# The icon is drawn by the binary itself, from the same picture as the window's.
icon_dir="$(mktemp -d)"
"$program" --icon "$icon_dir" > /dev/null
cp "$icon_dir/FormigaBattle.icns" "$app_dir/Contents/Resources/FormigaBattle.icns"
rm -rf "$icon_dir"

# The battle version comes from the binary itself, so the bundle can never claim another.
battle_version="$("$program" --battle-version)"
plist="$app_dir/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build_number" "$plist"
/usr/libexec/PlistBuddy -c "Set :FormigaBattleVersion $battle_version" "$plist"

if [[ -n "${FORMIGA_CODESIGN_IDENTITY:-}" ]]; then
    codesign --force --deep --options runtime --timestamp \
        --sign "$FORMIGA_CODESIGN_IDENTITY" "$app_dir"
else
    codesign --force --deep --sign - "$app_dir"
fi

ditto -c -k --sequesterRsrc --keepParent "$app_dir" "$archive"
mkdir -p "$dmg_staging"
ditto "$app_dir" "$dmg_staging/Formiga Battle.app"
ln -s /Applications "$dmg_staging/Applications"
cp "$repo_dir/packaging/macos/README.txt" "$dmg_staging/Read Me.txt"
hdiutil create -volname "Formiga Battle" -srcfolder "$dmg_staging" -ov -format UDZO "$disk_image"

if [[ -n "${FORMIGA_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$disk_image" --keychain-profile "$FORMIGA_NOTARY_PROFILE" --wait
    xcrun stapler staple "$disk_image"
fi

# Checksums name the file alone, so `shasum -c` works wherever the files are downloaded to.
(cd "$dist_dir" && for file in "$archive" "$disk_image"; do
    name="$(basename "$file")"
    shasum -a 256 "$name" > "$name.sha256"
done)

echo "Packaged $disk_image and $archive (battle version $battle_version)"
echo "Program digest: $(shasum -a 256 "$program" | cut -d ' ' -f 1)"
