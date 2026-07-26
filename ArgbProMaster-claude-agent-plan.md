================================================================================
ROLE & OBJECTIVE
================================================================================
You are an expert Systems & UI/UX Software Engineer specializing in Rust.
Your task is to build a modern, beginner-friendly, high-aesthetic Rust workspace consisting of:
1. 'configurator_gui': A beautiful, intuitive, feature-rich GUI built with `egui` / `eframe` that feels like a polished gamer utility with instant live previews, custom presets, and helpful tooltips.
2. 'thermal_daemon': An invisible, zero-overhead background daemon that reads PC thermals and renders high-speed ARGB animation frames via the OpenRGB SDK.

================================================================================
HARDWARE SPECIFICATIONS & HARDWARE MAPPING
================================================================================
- Motherboard: ASUS Prime X670-P WiFi (BIOS 3854)
- CPU: AMD Ryzen 9 7950X3D
- GPU: NVIDIA GeForce RTX 5070 Ti
- Header / Port 2 (ADD_GEN2_2): 72 LEDs total (3x Teucer strips) — Front Fans [DEFAULT: GPU]
- Header / Port 3 (ADD_GEN2_3): 96 LEDs total (4x Teucer strips) — Bottom Underglow [DEFAULT: CPU]
- DRAM (SMBus): Corsair Vengeance RGB DDR5 (2x10 = 20 LEDs total across 2 DIMM slots) [DEFAULT: CPU]

================================================================================
GUI DESIGN & USER EXPERIENCE REQUIREMENTS ('configurator_gui')
================================================================================
Design the GUI to be extremely approachable, sleek, dark-themed, and packed with controls:

1. TABBED / SECTIONED LAYOUT:
   - 🎨 Presets & Themes: One-click visual style switches.
   - ⚡ Port & Target Assignment: Visual dropdowns to link hardware zones to thermal sources.
   - 🌡️ Thermal Response Curves: Simple min/max temp sliders and color pickers.
   - ⚙️ Advanced Settings: Animation FPS, smoothing speed, and LED brightness caps.

2. BUILT-IN LIGHTING PRESET ENGINE:
   Provide pre-configured one-click presets that automatically update settings:
   - 🔥 "Thermal Alert" (Cold = Cyan/Blue, Warm = Purple, Hot = Crimson Red)
   - 🏙️ "Cyberpunk 2077" (Neon Yellow & Magenta dual-tone wave)
   - 🌴 "Vaporwave Sunset" (Pastel Pink, Purple, and Teal gradient)
   - ❄️ "Ice & Fire" (Port 3 stays Icy Blue, Port 2 turns Fiery Orange with GPU load)
   - 🥷 "Stealth Dark" (Dim ambient accent lighting at 15% brightness)
   - 🌈 "Smooth Spectrum Wave" (Classic rainbow wave independent of thermals)
   - 💾 "Custom Presets": User can name, save, and reload custom lighting profiles.

3. VIRTUAL LIVE PREVIEW PANEL:
   - Render a real-time virtual LED bar preview directly in the GUI so the user can see what their animations look like before saving.
   - Include mock temperature sliders so users can test how the lights shift when CPU/GPU temperatures rise!

4. FRIENDLY UX CONTROLS & TOOLTIPS:
   - Hover tooltips on EVERY slider explaining what it does (e.g., "Easing Speed: Controls how slowly colors bleed when temperatures spike").
   - Brightness Slider: Soft-capped at 70% with a "Safety Power Lock" toggle.
   - "Apply & Save": Saves choices to `settings.json` and closes the GUI cleanly.
   - "Run in Background": Keeps daemon active and quietly minimizes GUI to system tray.

================================================================================
CONFIGURATION SCHEMA (`settings.json`)
================================================================================
{
  "active_preset": "Thermal Alert",
  "global_brightness": 0.70,
  "animation_fps": 15,
  "smoothing_speed": 0.05,
  "effects_mode": "ThermalWave", // Options: "ThermalWave", "GradientPulse", "Breathing", "Solid"
  "cpu_temp_min": 40.0,
  "cpu_temp_max": 85.0,
  "gpu_temp_min": 30.0,
  "gpu_temp_max": 75.0,
  "colors": {
    "cold_color": [0, 255, 200],    // RGB Array
    "warm_color": [180, 0, 255],
    "hot_color": [255, 10, 10]
  },
  "ports": {
    "port_2": {
      "enabled": true,
      "led_count": 72,
      "target_source": "GPU",
      "description": "Front Fans (3 Strips)"
    },
    "port_3": {
      "enabled": true,
      "led_count": 96,
      "target_source": "CPU",
      "description": "Bottom Underglow (4 Strips)"
    },
    "ram": {
      "enabled": true,
      "sticks": 2,
      "leds_per_stick": 10,
      "total_led_count": 20,
      "target_source": "CPU",
      "description": "Corsair Vengeance RGB DDR5"
    }
  }
}

================================================================================
HEADLESS BACKGROUND DAEMON (`thermal_daemon`)
================================================================================
- Lightweight background process running without console window allocation.
- Polling Loop: Reads CPU and GPU metrics from MSI Afterburner Shared Memory (`MAHMSharedMemory`) via Win32 memory-mapped file API.
- Animation Engine:
  * Computes spatial sine wave gradients dynamically based on active preset settings.
  * Applies exponential smoothing (`working_temp += (target_temp - working_temp) * smoothing_speed`) to guarantee fluid transitions.
  * Dynamically re-reads `settings.json` whenever the file modification timestamp updates.
- OpenRGB Client: Sends RGB byte arrays over local TCP socket (Port 6742) to:
  * Port 2 (72 LEDs) -> Rendered via smoothed GPU temp.
  * Port 3 (96 LEDs) -> Rendered via smoothed CPU temp.
  * RAM (20 LEDs across 2 sticks) -> Rendered sequentially across DIMM 1 and DIMM 2 via smoothed CPU temp.

================================================================================
DELIVERABLES
================================================================================
1. Full Rust Cargo workspace structure with `configurator_gui` and `thermal_daemon`.
2. Fully stylized `egui` code with dark theme, tooltips, preset selector, and thermal simulation sliders.
3. Win32 MSI Afterburner shared memory bindings & OpenRGB TCP client integration.
4. Clean build outputs with zero compiler errors via `cargo build --release`.
================================================================================