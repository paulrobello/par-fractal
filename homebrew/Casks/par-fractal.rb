cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.8.0"
sha256 arm:   "5a7447d9c22e555b8af9016780706004331fcf0c284f6754797b8e32d234afc0",
       intel: "beeb2a9cba1f0a0474e22e9ba5ceb1c8d20cb8741b06075546dcacc6eb9e739f"

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
