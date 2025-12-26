cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.8.1"
sha256 arm:   "2adec3aa537c78588134fb89f03ca0924c0c4931334f150b24d4c34693ac9303",
       intel: "ca8ac9c068c7e27e63056b7051bc48a79174e986c45fb628dcbfb401a49b8082"

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
