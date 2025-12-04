cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.7.1"
sha256 arm:   "3652ccdf5893f1df0531e3ce640cc22493f8c7d72b842ba185a15ff1e264e39c",
       intel: "de101728fe65bd255014bf538b611f2c2f4c8f0bd149839bbcd2bb27c718bc1b"

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
