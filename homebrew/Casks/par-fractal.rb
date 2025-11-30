cask "par-fractal" do
  arch arm: "aarch64", intel: 0019dfc4b32d63c1392aa264aed2253c1e0c2fb09216f8e2cc269bbfb8bb49b5

  version 0.6.1
  sha256 arm:   0019dfc4b32d63c1392aa264aed2253c1e0c2fb09216f8e2cc269bbfb8bb49b5
         intel: 0019dfc4b32d63c1392aa264aed2253c1e0c2fb09216f8e2cc269bbfb8bb49b5

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
