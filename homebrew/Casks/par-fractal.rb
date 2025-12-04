cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.7.0"
sha256 arm:   "33af66330da0fd957c1192f4eea389dfbb4af3a5f3a63d4ad6674fb91eff96d8",
       intel: "2f7d4d1c0045b7390c5e490a113fb64e4998b0b408ccec5171ec1674380c39fa"

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
