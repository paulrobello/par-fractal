cask "par-fractal" do
arch arm: "aarch64", intel: "x86_64"

version "0.8.3"
sha256 arm:   "d0e2fc6bb6c7db4dae8af02ab1567fc46905b66fa85f83dbc667f9614c4579a3",
       intel: "e9e2690c8d0fbf267a0449a1c88f5197f4db9e73451819e1f180ec90f90e4903"

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
