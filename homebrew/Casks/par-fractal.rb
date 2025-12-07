cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.7.2"
sha256 arm:   "e776558cf67a5e4ad16d0a9b984e33b12bcf7c4da4379dccf7d0774e3b6859cf",
       intel: "c1ac0234ef72c8b2c4f71d965171cc262551488eed8856b059024b0b27339d92"

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
