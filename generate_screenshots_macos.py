#!/usr/bin/env python3
"""Generate macOS App Store screenshots for eudamed2swissdamed.

Resolution: 2880x1800 — PNG format.
Light theme to match the actual app (egui Visuals::light()).
macOS-style title bar with traffic light buttons.
"""

from PIL import Image, ImageDraw, ImageFont
import os

W, H = 2880, 1800

# Light theme colors (matching egui light mode)
BG = (236, 236, 236)            # macOS window background
PANEL = (248, 248, 248)         # content area bg
WIDGET_BG = (255, 255, 255)     # input field bg
BORDER = (200, 200, 200)        # borders
TEXT = (30, 30, 30)             # primary text
TEXT_DIM = (140, 140, 140)      # dimmed/hint text
ACCENT = (0, 102, 204)         # blue accent
GREEN = (16, 150, 60)          # success
BUTTON_BG = (0, 102, 204)      # button
BUTTON_BG_DISABLED = (200, 200, 200)
BUTTON_TEXT = (255, 255, 255)
BUTTON_TEXT_DISABLED = (140, 140, 140)
SEPARATOR = (210, 210, 210)
CHECKBOX_ON = ACCENT
TITLE_BAR = (232, 232, 232)        # macOS light title bar
TITLE_BAR_BORDER = (200, 200, 200)
LOG_BG = (252, 252, 252)           # log area bg

# Traffic light colors
TL_CLOSE = (255, 95, 87)
TL_MINIMIZE = (255, 189, 46)
TL_MAXIMIZE = (39, 201, 63)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ICON_PATH = os.path.join(SCRIPT_DIR, "assets", "icon_256x256.png")
OUT_DIR = os.path.join(SCRIPT_DIR, "screenshots", "macos")

# Font loading — macOS fonts
def get_fonts():
    mono_paths = [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/Library/Fonts/SF-Mono-Regular.otf",
    ]
    mono = None
    for p in mono_paths:
        if os.path.exists(p):
            try:
                mono = ImageFont.truetype(p, 30)
                break
            except:
                pass

    sans_paths = [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/HelveticaNeue.ttc",
    ]
    sans = None
    for p in sans_paths:
        if os.path.exists(p):
            try:
                sans = ImageFont.truetype(p, 34)
                break
            except:
                pass

    if mono is None:
        mono = ImageFont.load_default()
    if sans is None:
        sans = mono
    return sans, mono

SANS, MONO = get_fonts()

def sized_font(paths, size):
    for p in paths:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except:
                pass
    return SANS

SANS_PATHS = [
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
    "/System/Library/Fonts/HelveticaNeue.ttc",
]
MONO_PATHS = [
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/Library/Fonts/SF-Mono-Regular.otf",
]
BOLD_PATHS = [
    "/System/Library/Fonts/SFNSTextBold.ttf",
    "/Library/Fonts/SF-Pro-Text-Bold.otf",
]

SANS_SMALL = sized_font(SANS_PATHS, 28)
SANS_LABEL = sized_font(SANS_PATHS, 34)
SANS_HEADING = sized_font(SANS_PATHS, 48)
SANS_TITLEBAR = sized_font(SANS_PATHS, 28)
MONO_LOG = sized_font(MONO_PATHS, 26)


def draw_macos_titlebar(draw, img, title="eudamed2swissdamed"):
    """Draw macOS-style title bar with traffic light buttons."""
    bar_h = 56

    # Title bar background
    draw.rectangle([0, 0, W, bar_h], fill=TITLE_BAR)
    draw.line([0, bar_h, W, bar_h], fill=TITLE_BAR_BORDER, width=1)

    # Traffic light buttons
    tl_y = bar_h // 2
    tl_r = 12
    tl_start_x = 30
    tl_spacing = 40

    # Close (red)
    draw.ellipse([tl_start_x - tl_r, tl_y - tl_r, tl_start_x + tl_r, tl_y + tl_r],
                 fill=TL_CLOSE)
    # Minimize (yellow)
    mx = tl_start_x + tl_spacing
    draw.ellipse([mx - tl_r, tl_y - tl_r, mx + tl_r, tl_y + tl_r],
                 fill=TL_MINIMIZE)
    # Maximize (green)
    zx = tl_start_x + 2 * tl_spacing
    draw.ellipse([zx - tl_r, tl_y - tl_r, zx + tl_r, tl_y + tl_r],
                 fill=TL_MAXIMIZE)

    # Title text centered
    bbox = draw.textbbox((0, 0), title, font=SANS_TITLEBAR)
    tw = bbox[2] - bbox[0]
    draw.text(((W - tw) // 2, 14), title, fill=TEXT, font=SANS_TITLEBAR)


def draw_rounded_rect(draw, xy, radius, fill=None, outline=None):
    draw.rounded_rectangle(xy, radius=radius, fill=fill, outline=outline)


def draw_text_input(draw, x, y, w, h, text="", hint="", font=None):
    if font is None:
        font = SANS_LABEL
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=6, fill=WIDGET_BG, outline=BORDER)
    if text:
        draw.text((x+12, y+8), text, fill=TEXT, font=font)
    elif hint:
        draw.text((x+12, y+8), hint, fill=TEXT_DIM, font=font)


def draw_multiline_input(draw, x, y, w, h, lines=None, hint_lines=None, font=None):
    if font is None:
        font = SANS_LABEL
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=6, fill=WIDGET_BG, outline=BORDER)
    if lines:
        for i, line in enumerate(lines):
            draw.text((x+12, y+10+i*42), line, fill=TEXT, font=font)
    elif hint_lines:
        for i, line in enumerate(hint_lines):
            draw.text((x+12, y+10+i*42), line, fill=TEXT_DIM, font=font)


def draw_button(draw, x, y, w, h, text, enabled=True):
    bg = BUTTON_BG if enabled else BUTTON_BG_DISABLED
    fg = BUTTON_TEXT if enabled else BUTTON_TEXT_DISABLED
    border = ACCENT if enabled else BORDER
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=6, fill=bg, outline=border)
    bbox = draw.textbbox((0, 0), text, font=SANS_LABEL)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    draw.text((x+(w-tw)//2, y+(h-th)//2 - 2), text, fill=fg, font=SANS_LABEL)


def draw_checkbox(draw, x, y, checked=False, label=""):
    s = 28
    draw_rounded_rect(draw, [x, y, x+s, y+s], radius=5,
                       fill=CHECKBOX_ON if checked else WIDGET_BG, outline=BORDER)
    if checked:
        draw.line([x+6, y+14, x+11, y+21], fill=(255, 255, 255), width=3)
        draw.line([x+11, y+21, x+22, y+7], fill=(255, 255, 255), width=3)
    draw.text((x+s+12, y-2), label, fill=TEXT, font=SANS_LABEL)


def draw_log_area(draw, x, y, w, h, lines, font=None):
    if font is None:
        font = MONO_LOG
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=6, fill=LOG_BG, outline=BORDER)
    clip_y = y + 12
    for line in lines:
        if clip_y + 34 > y + h - 12:
            break
        color = TEXT
        if line.startswith("=== DONE"):
            color = GREEN
        elif line.startswith("=== FAILED"):
            color = (220, 40, 40)
        elif line.startswith("["):
            color = ACCENT
        draw.text((x+14, clip_y), line, fill=color, font=font)
        clip_y += 34


def add_icon(img, x, y, size=84):
    try:
        icon = Image.open(ICON_PATH).convert("RGBA")
        icon = icon.resize((size, size), Image.LANCZOS)
        img.paste(icon, (x, y), icon)
    except:
        pass


def screenshot_1_main():
    """Screenshot 1: Main window, empty state with hint text."""
    img = Image.new("RGB", (W, H), PANEL)
    draw = ImageDraw.Draw(img)
    draw_macos_titlebar(draw, img)

    margin = 60
    y = 86

    add_icon(img, W - margin - 84, y + 2, 84)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 76

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 50
    draw_multiline_input(draw, margin, y, W - 2*margin, 140,
                         hint_lines=["CH-MF-000023141", "CH-MF-000012345"])
    y += 162

    draw.text((margin, y+4), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+260, y, 150, 48, hint="all")
    draw_checkbox(draw, margin+460, y+12, checked=False, label="Dry run (no push)")
    y += 72

    draw.text((margin, y), u"\u25B6 Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 56

    draw_button(draw, margin, y, 440, 56, "Download & Push", enabled=False)
    y += 82

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 18

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 46

    draw_log_area(draw, margin, y, W-2*margin, H-y-50, [])

    img.save(os.path.join(OUT_DIR, "screenshot_1_main.png"))
    print("Saved screenshot_1_main.png")


def screenshot_2_running():
    """Screenshot 2: Download running with log output."""
    img = Image.new("RGB", (W, H), PANEL)
    draw = ImageDraw.Draw(img)
    draw_macos_titlebar(draw, img)

    margin = 60
    y = 86

    add_icon(img, W - margin - 84, y + 2, 84)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 76

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 50
    draw_multiline_input(draw, margin, y, W - 2*margin, 140,
                         lines=["CH-MF-000023141"])
    y += 162

    draw.text((margin, y+4), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+260, y, 150, 48, text="50")
    draw_checkbox(draw, margin+460, y+12, checked=False, label="Dry run (no push)")
    y += 72

    draw.text((margin, y), u"\u25B6 Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 56

    draw_button(draw, margin, y, 440, 56, "Running...", enabled=False)
    y += 82

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 18

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 46

    log_lines = [
        "[Download] Starting pipeline for 1 SRN(s), limit 50 per SRN",
        "[Download] Fetching listings from EUDAMED...",
        "[Download] Fetching listing page 0 (pageSize=300)...",
        "[Download] Found 185 devices on page 0",
        "[Download] Limiting to 50 devices",
        "[Download] 50 UUIDs extracted from listings",
        "[Download] Downloading 50 detail files...",
        "[Download] Downloading detail 1/50: 4f1e3733-2987-4d3b-...",
        "[Download] Downloading detail 2/50: 7cd1d81c-b335-4f95-...",
        "[Download] Downloading detail 3/50: a87f1218-0aa5-4427-...",
        "[Download] Downloading detail 4/50: 3c298386-e47c-411a-...",
        "[Download] Downloading detail 5/50: 9bd4b6bb-3065-4558-...",
        "[Download] Downloading Basic UDI-DI data...",
        "[Download] Downloading basic UDI-DI 1/50...",
        "[Download] Downloading basic UDI-DI 2/50...",
    ]
    draw_log_area(draw, margin, y, W-2*margin, H-y-50, log_lines)

    img.save(os.path.join(OUT_DIR, "screenshot_2_running.png"))
    print("Saved screenshot_2_running.png")


def screenshot_3_done():
    """Screenshot 3: Completed pipeline with success summary."""
    img = Image.new("RGB", (W, H), PANEL)
    draw = ImageDraw.Draw(img)
    draw_macos_titlebar(draw, img)

    margin = 60
    y = 86

    add_icon(img, W - margin - 84, y + 2, 84)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 76

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 50
    draw_multiline_input(draw, margin, y, W - 2*margin, 140,
                         lines=["CH-MF-000023141"])
    y += 162

    draw.text((margin, y+4), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+260, y, 150, 48, text="50")
    draw_checkbox(draw, margin+460, y+12, checked=False, label="Dry run (no push)")
    y += 72

    draw.text((margin, y), u"\u25B6 Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 56

    draw_button(draw, margin, y, 440, 56, "Download & Push", enabled=True)
    y += 82

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 18

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 46

    log_lines = [
        "[Download] Starting pipeline for 1 SRN(s), limit 50 per SRN",
        "[Download] Fetching listings from EUDAMED...",
        "[Download] Found 185 devices, limiting to 50",
        "[Download] 50 UUIDs extracted from listings",
        "[Download] Downloaded 50/50 details + 50/50 basic UDI-DI",
        "[Download] Completeness check + retry...",
        "[Download] All 50 devices complete",
        "[Version DB] 38 new/changed devices tracked",
        "[Push] Authenticating with Swissdamed...",
        "[Push] Pushing 50 devices...",
        "[Push] Submitted: 48, Failed: 0, Skipped: 2",
        "",
        "=== DONE === 48 submitted, 0 failed, 2 skipped",
    ]
    draw_log_area(draw, margin, y, W-2*margin, H-y-50, log_lines)

    img.save(os.path.join(OUT_DIR, "screenshot_3_done.png"))
    print("Saved screenshot_3_done.png")


def screenshot_4_credentials():
    """Screenshot 4: Swissdamed credentials expanded."""
    img = Image.new("RGB", (W, H), PANEL)
    draw = ImageDraw.Draw(img)
    draw_macos_titlebar(draw, img)

    margin = 60
    y = 86

    add_icon(img, W - margin - 84, y + 2, 84)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 76

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 50
    draw_multiline_input(draw, margin, y, W - 2*margin, 140,
                         lines=["CH-MF-000023141", "DE-MF-000017808"])
    y += 162

    draw.text((margin, y+4), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+260, y, 150, 48, hint="all")
    draw_checkbox(draw, margin+460, y+12, checked=False, label="Dry run (no push)")
    y += 72

    draw.text((margin, y), u"\u25BC Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 50

    draw.text((margin+16, y+6), "Client ID:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+230, y, 560, 48, text="my-client-id-xxxxx")
    y += 64

    draw.text((margin+16, y+6), "Client Secret:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+230, y, 560, 48, text="****************")
    y += 64

    draw.text((margin+16, y+6), "API Base URL:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+230, y, 560, 48, text="https://playground.swissdamed.ch")
    y += 78

    draw_button(draw, margin, y, 440, 56, "Download & Push", enabled=True)
    y += 82

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 18

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 46

    draw_log_area(draw, margin, y, W-2*margin, H-y-50, [])

    img.save(os.path.join(OUT_DIR, "screenshot_4_credentials.png"))
    print("Saved screenshot_4_credentials.png")


def screenshot_5_dryrun():
    """Screenshot 5: Dry run completed (download & preview only)."""
    img = Image.new("RGB", (W, H), PANEL)
    draw = ImageDraw.Draw(img)
    draw_macos_titlebar(draw, img)

    margin = 60
    y = 86

    add_icon(img, W - margin - 84, y + 2, 84)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 76

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 50
    draw_multiline_input(draw, margin, y, W - 2*margin, 140,
                         lines=["CH-MF-000023141"])
    y += 162

    draw.text((margin, y+4), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+260, y, 150, 48, text="10")
    draw_checkbox(draw, margin+460, y+12, checked=True, label="Dry run (no push)")
    y += 72

    draw.text((margin, y), u"\u25B6 Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 56

    draw_button(draw, margin, y, 440, 56, "Download & Preview", enabled=True)
    y += 82

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 18

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 46

    log_lines = [
        "[Download] Starting pipeline for 1 SRN(s), limit 10 per SRN",
        "[Download] Fetching listings from EUDAMED...",
        "[Download] Found 185 devices, limiting to 10",
        "[Download] 10 UUIDs extracted from listings",
        "[Download] Downloaded 10/10 details + 10/10 basic UDI-DI",
        "[Download] Completeness check + retry...",
        "[Download] All 10 devices complete",
        "[Version DB] 10 new/changed devices tracked",
        "",
        "=== DONE === Dry run complete. 10 devices downloaded, ready to push.",
    ]
    draw_log_area(draw, margin, y, W-2*margin, H-y-50, log_lines)

    img.save(os.path.join(OUT_DIR, "screenshot_5_dryrun.png"))
    print("Saved screenshot_5_dryrun.png")


if __name__ == "__main__":
    os.makedirs(OUT_DIR, exist_ok=True)
    screenshot_1_main()
    screenshot_2_running()
    screenshot_3_done()
    screenshot_4_credentials()
    screenshot_5_dryrun()
    print(f"\nAll screenshots saved to {OUT_DIR}/")
    print("Size: 2880x1800 — PNG format")
