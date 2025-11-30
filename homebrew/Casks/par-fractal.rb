cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.6.1"
sha256 arm:   "e6acffbaa6a734a681e4176cb3d122a2899b4fc5f5216409f96032221725308b",
       intel: "36618d1cb09ef3e248b8683d49a87495814d54fb2deb3a7085e546601719c1a7"

url "https://github.com/paulrobello/par-fractal/releases/download/v#{version}/par-fractal-macos-#{arch}.zip"
name "par-fractal"
desc "Cross-platform GPU-accelerated fractal renderer with 2D and 3D support"
homepage "https://github.com/paulrobello/par-fractal"

depends_on macos: ">= :catalina"

livecheck do
  url :homepage
  strategy :github_latest
end

app "par-fractal.app"

zap trash: [
  "~/Library/Application Support/par-fractal",
  "~/Library/Preferences/com.paulrobello.par-fractal.plist",
  "~/Library/Saved Application State/com.paulrobello.par-fractal.savedState",
]
end
