cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.8.0"
sha256 arm:   "e847d7e416b8cfe5ec870a68f90478387699e5b0bc7809dc00c50c6d44912d29",
       intel: "4cb2392b47e7a159ee67b6ab0d7ed68490cfa95bbcac08c637beae456efdef2b"

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
