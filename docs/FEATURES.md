# Features

Complete feature documentation for Par Fractal's rendering capabilities, effects, and tools.

## Table of Contents
- [Fractal Types](#fractal-types)
- [Rendering Capabilities](#rendering-capabilities)
- [Screenshot and Recording](#screenshot-and-recording)
- [Productivity Tools](#productivity-tools)
- [Performance Optimization](#performance-optimization)
- [Color System](#color-system)

## Fractal Types

### 2D Fractals (12 types)

Par Fractal supports 12 different 2D escape-time fractals:

- **Mandelbrot2D** - The classic fractal with infinite detail at all zoom levels
- **Julia2D** - Symmetric variations with customizable complex constant parameters
- **BurningShip2D** - Dramatic ship-like patterns with sharp edges
- **Tricorn2D** - Conjugate Mandelbrot with unique symmetry
- **Phoenix2D** - Multi-parameter fractal with flowing patterns
- **Celtic2D** - Ornate patterns resembling Celtic knots
- **Newton2D** - Newton's method fractal with root-finding patterns
- **Lyapunov2D** - Chaotic sequences visualization
- **Nova2D** - Newton-Raphson hybrid fractal
- **Magnet2D** - Magnetic field-inspired patterns
- **Collatz2D** - Collatz conjecture visualization
- **Sierpinski2D** - Triangle-based recursive pattern

### 3D Fractals (12 types)

Par Fractal supports 12 different 3D ray-marched fractals:

- **Mandelbulb3D** - 3D extension of Mandelbrot set with configurable power (typically 8)
- **MengerSponge3D** - Recursive cube structure with self-similarity
- **SierpinskiPyramid3D** - 3D tetrahedral recursive pattern
- **JuliaSet3D** - 3D Julia set with adjustable parameters
- **Mandelbox3D** - Box-folding fractal with scale and fold parameters
- **TgladFormula3D** - Advanced IFS fractal with complex folding
- **OctahedralIFS3D** - Octahedral symmetry iterated function system
- **IcosahedralIFS3D** - Icosahedral symmetry iterated function system
- **ApollonianGasket3D** - Circle-packing fractal in 3D
- **Kleinian3D** - Kleinian group limit set visualization
- **HybridMandelbulbJulia3D** - Combination of Mandelbulb and Julia patterns
- **QuaternionCubic3D** - Quaternion-based cubic iteration

## Rendering Capabilities

### 2D Mode

**Visual Quality:**
- Smooth anti-aliased rendering
- Efficient GPU-based escape-time algorithm
- Real-time parameter adjustment
- Adaptive quality settings

**Navigation:**
- Pan and zoom with mouse controls
- Zoom-to-cursor positioning
- Interactive exploration with unlimited zoom depth
- Smooth, continuous zoom with automatic detail enhancement

### 3D Mode

#### Shading Models

**Blinn-Phong Shading**
- Classic lighting model
- Fast performance
- Specular highlights
- Good for real-time interaction

**Physically Based Rendering (PBR)**
- Realistic material appearance
- Energy-conserving lighting
- Advanced surface properties
- Suitable for final renders

#### Visual Effects

**Ambient Occlusion (AO)**
- Depth perception enhancement
- Contact shadows in crevices
- Configurable intensity (0.0-10.0)
- Adjustable step size for precision
- GPU-accelerated ray marching
- Toggle on/off support

**Shadow System**
- **Off** - No shadows (fastest)
- **Hard Shadows** - Sharp shadow edges
- **Soft Shadows** - Realistic penumbra with configurable softness
- Configurable shadow samples (16-128)
- Distance-based attenuation
- Shadow step factor for performance tuning
- Maximum shadow distance control

**Depth of Field (DoF)**
- Camera-like focus effect
- Adjustable focal length
- Configurable aperture size
- Bokeh-like blur
- Sample count control (1-8)
- Toggle on/off support

**Distance Fog**
- Three fog modes:
  - **Linear** - Linear distance falloff
  - **Exponential** - Natural exponential falloff
  - **Quadratic** - Exponential squared falloff
- Configurable density (0.0-1.0)
- Custom fog color
- Distance-based attenuation
- Enhances depth perception
- Toggle on/off support

#### Material System

**Surface Properties:**
- **Roughness** - Controls surface microsurface detail
  - Low: Smooth, mirror-like
  - High: Rough, diffuse
- **Metallic** - Defines material type
  - 0.0: Dielectric (non-metal)
  - 1.0: Metallic
- **Albedo Color** - Base surface color
  - Full RGB control
  - Interactive color picker

#### Floor System

**Checkered Floor:**
- Toggle floor on/off
- Adjustable floor height
- Two-color checkered pattern
- Custom colors for both squares
- Floor reflections (optional)
- Configurable reflection strength
- Automatic integration with lighting and fog

#### Lighting System

**Light Configuration:**
- Adjustable light intensity (0.0-10.0, default 3.0)
- Ambient light control (0.0-1.0, default 0.15)
- Light position via azimuth (horizontal angle)
- Light elevation (vertical angle)
- Real-time light position updates
- Works with both Blinn-Phong and PBR shading

#### Ray Marching Parameters

**Advanced Controls:**
- **Max Steps** - Maximum ray marching iterations (default 200)
- **Min Distance** - Surface precision threshold (default 0.00035)
- **Max Distance** - Ray marching cutoff distance (default 100.0)
- **Step Multiplier** - Global step size adjustment
- **Adaptive Step** - Enable/disable adaptive step sizing
- **Fixed Step Size** - Manual step size when adaptive is off

**Fractal-Specific Parameters:**
- **Power** - Mandelbulb/Julia power (default 8.0)
- **Scale** - Fractal scale factor
- **Fold** - Box-folding parameter (Mandelbox, etc.)
- **Min Radius** - Minimum folding radius
- **Max Iterations** - Escape-time iterations (2D) or DE iterations (3D)

#### Camera Controls

**Movement:**
- WASD - Forward/back, left/right
- Space/Shift - Up/down
- Smooth camera interpolation
- Adjustable movement speed (default 2.0)
- Mouse wheel for speed adjustment

**View Control:**
- Mouse drag for camera rotation
- Configurable field of view (FOV)
- Reset to default view
- Camera bookmark system (save/load positions)
- Auto-orbit mode with adjustable speed
- Smooth camera transitions

## Screenshot and Recording

### Screenshot Features

**Instant Capture:**
- **F12 Hotkey** - Quick PNG screenshots
- Toast notifications with click-to-open
- Automatic filename generation with fractal type
- Timestamp-based organization

**High-Resolution Rendering:**
- Custom resolution support
- Common presets (HD, 2K, 4K, 8K)
- Independent of window size
- GPU-accelerated rendering

**Monitor-Specific Rendering:**
- Auto-detect connected monitors
- Render at native resolution
- Multi-monitor support
- Optimal quality for each display

### Video Recording

**Recording Capabilities:**
- MP4 video capture (H.264 codec)
- WebM video support (VP9 codec)
- GIF animation support
- Configurable frame rate (default 60 FPS)

**Recording Features:**
- Start/stop controls via UI
- Real-time frame capture
- Requires FFmpeg for encoding
- Automatic filename generation with timestamp

## Productivity Tools

### Command Palette

**Quick Access:**
- **Ctrl/Cmd+P** - Open palette
- Fuzzy search
- Keyboard-driven workflow
- All commands accessible

**Features:**
- Command history
- Keyboard shortcuts shown
- Category organization
- Instant execution

### Preset System

**Preset Management:**
- Save complete fractal configurations
- Load saved presets
- Category organization (All, User, Showcase)
- Searchable preset gallery
- Import/export YAML functionality
- Preset metadata (name, description, category)

**Built-in Presets:**
- Showcase presets included
- User preset collection
- Quick preset switching via UI
- Preserves all parameters:
  - Fractal type and parameters
  - Camera position (3D)
  - Color settings
  - Effects and quality
  - Lighting configuration

### Camera Bookmarks

**3D Navigation Aid:**
- Save camera positions
- Named bookmarks
- Quick restoration
- Per-fractal storage

**Use Cases:**
- Mark interesting views
- Tour planning
- Screenshot preparation
- Exploration waypoints

### Color Palette System

**Built-in Palettes (21 total):**

*Classic Palettes:*
- **Fire** - Black to purple to red to orange to yellow
- **Ocean** - Deep blue to cyan gradients
- **Rainbow** - Full spectrum color cycle
- **Forest** - Natural green tones
- **Sunset** - Purple to pink to orange to yellow
- **Grayscale** - Black to white monochrome

*Scientific Visualization (Perceptually Uniform):*
- **Viridis** - Purple to teal to green to yellow
- **Plasma** - Blue to purple to magenta to orange to yellow
- **Inferno** - Black to purple to red to orange to yellow
- **Magma** - Black to purple to pink-red to orange to pale yellow

*Specialty Palettes:*
- **Copper** - Black to brown to copper metallic
- **Cool** - Cyan to blue to purple to magenta
- **Hot** - Black to red to orange to yellow to white
- **Neon** - Vibrant electric colors
- **Purple Dream** - Purple and violet variations
- **Earth** - Brown, tan, and earth tones
- **Ice** - Cool blue and white tones
- **Lava** - Molten lava colors
- **Galaxy** - Deep space purples and blues
- **Mint** - Fresh green and mint tones
- **Cherry** - Red, pink, and cherry variations

**Custom Palettes:**
- Create 5-color gradient palettes
- Save and load custom palettes
- Import from YAML files
- Export for sharing
- Interactive color picker

**Palette Features:**
- Real-time preview
- Smooth 5-point interpolation
- Palette offset control
- Cycle through palettes with **P** key
- Palette animation (auto-cycling)

### Undo/Redo System

**History Management:**
- Full parameter change history
- **Ctrl/Cmd+Z** - Undo
- **Ctrl/Cmd+Shift+Z** - Redo
- Configurable history depth (default 50)

**Tracked Changes:**
- Fractal type switches
- Parameter adjustments
- Camera movements (3D)
- Effect toggles
- Color/palette changes

### Randomization

**Creative Exploration:**
- Randomize fractal type
- Randomize color palette
- Randomize color mode
- Randomize 2D parameters (Julia constant, iterations)
- Randomize 3D parameters (power, scale, fold, etc.)
- Randomize lighting (intensity, ambient)
- Randomize effects (AO, shadows, fog)
- Randomize materials (roughness, metallic)
- One-click creative discovery

### CLI Options

**Command-Line Features:**
- `--clear-settings` - Reset all saved preferences
- `--preset "name"` - Load specific preset on startup
- `--list-presets` - List available presets
- `--screenshot-delay N` - Take screenshot after N seconds
- `--exit-delay N` - Exit application after N seconds
- Useful for automation and batch rendering

## Performance Optimization

### Level of Detail (LOD) System

**Dynamic Quality:**
- Automatic quality adjustment
- Camera movement detection
- Smooth quality transitions
- Performance maintenance

**LOD Behavior:**
- Reduce quality during motion
- Restore quality when still
- Configurable thresholds
- User override available

### Quality Profiles

**LOD Quality Levels:**
- **Ultra** - 325 ray steps, 128 shadow samples, 0.00035 precision, 8 DoF samples
- **High** - 250 ray steps, 64 shadow samples, 0.0007 precision, 4 DoF samples
- **Medium** - 175 ray steps, 32 shadow samples, 0.0015 precision, 2 DoF samples
- **Low** - 100 ray steps, 16 shadow samples, 0.003 precision, 1 DoF sample

**LOD Strategies:**
- **Distance** - Reduce quality based on camera distance from fractal
- **Motion** - Lower quality during camera movement
- **Performance** - Adjust quality to maintain target FPS
- **Hybrid** - Intelligently combine all strategies

**LOD Profiles:**
- **Balanced** - Good mix of quality and performance (60 FPS target)
- **Quality First** - Prioritize visuals, less aggressive (45 FPS target)
- **Performance First** - Prioritize FPS, aggressive reduction (75 FPS target)
- **Distance Only** - Only use distance-based LOD
- **Motion Only** - Only reduce quality during motion
- **Custom** - User-defined configuration

### GPU Selection

**Multi-GPU Support:**
- Automatic GPU detection
- Manual GPU selection
- Adapter information display
- Performance comparison

### Performance Monitoring

**Real-time Metrics:**
- FPS display overlay
- Frame time tracking
- Performance graphs
- Bottleneck identification

## Color System

### Color Modes

**Standard Coloring:**
- **Palette** - Standard palette-based coloring (default)
- **PerChannel** - Custom R/G/B channel mapping from different sources

**Debug & Visualization Modes:**
- **RaySteps** - Visualize number of ray marching steps
- **Normals** - Surface normal visualization
- **WorldPosition** - Color based on world coordinates
- **LocalPosition** - Color based on fractal-space coordinates
- **AmbientOcclusion** - Visualize AO values only
- **DistanceField** - Visualize distance estimator values
- **Depth** - Surface depth from camera
- **Convergence** - Escape time visualization (2D fractals)
- **LightingOnly** - Show only lighting without fractal coloring
- **ShadowMap** - Visualize shadow values
- **CameraDistanceLOD** - LOD zone visualization with colors
- **DistanceGrayscale** - Raw distance as grayscale

**Orbit Trap Coloring:**
- **OrbitTrapXYZ** - Color based on XYZ coordinates during iteration
- **OrbitTrapRadial** - Color based on radial distance during iteration
- Adjustable orbit trap scale

**Per-Channel Mapping:**
Custom channel sources for R/G/B:
- Iterations - Iteration/step count
- Distance - Distance to surface
- PositionX/Y/Z - Coordinate components
- Normal - Surface normal component
- AO - Ambient occlusion value
- Constant - Fixed value (0.0)

### Color Interpolation

**Smooth Gradients:**
- Linear interpolation between palette colors
- 5-point color gradient system
- Smooth anti-banding
- Real-time palette offset animation

### Color Controls

**Palette Options:**
- 21 built-in palettes + custom palettes
- Color offset adjustment (shift colors)
- Interactive palette switching (P key)
- Palette animation with speed control

**Post-Processing:**
- **Brightness** - Exposure adjustment
- **Contrast** - Contrast enhancement
- **Saturation** - Color intensity
- **Hue Shift** - Color rotation
- **Vignette** - Edge darkening effect
- **Bloom** - Glow effect with threshold and intensity
- **FXAA** - Fast approximate anti-aliasing

**Real-time Adjustment:**
- Interactive sliders
- Live preview
- Undo/redo support
- Preset integration

---

For implementation details, see the [Architecture Guide](ARCHITECTURE.md).
For controls reference, see the [Controls Guide](CONTROLS.md).
