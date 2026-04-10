#!/usr/bin/env python3
"""Generate Windows Store screenshots for eudamed2swissdamed.

Resolution: 3840x2160 (4K) — PNG format.
Light theme to match the actual app (egui Visuals::light()).
Windows-style title bar with minimize/maximize/close buttons.
"""

from PIL import Image, ImageDraw, ImageFont
import os

W, H = 3840, 2160

# Light theme colors (matching egui light mode)
BG = (248, 248, 248)            # window background
PANEL = (255, 255, 255)         # panel bg
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
SEPARATOR = (220, 220, 220)
CHECKBOX_ON = ACCENT
TITLE_BAR = (255, 255, 255)        # Windows 11 light title bar
TITLE_BAR_BORDER = (220, 220, 220)
CLOSE_HOVER = (232, 17, 35)        # Windows close button red
LOG_BG = (252, 252, 252)           # log area bg

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ICON_PATH = os.path.join(SCRIPT_DIR, "assets", "icon_256x256.png")
OUT_DIR = os.path.join(SCRIPT_DIR, "screenshots", "windows")

# Font loading — try Windows fonts first, then fallbacks
def get_fonts():
    mono_paths = [
        "C:/Windows/Fonts/consola.ttf",      # Consolas
        "C:/Windows/Fonts/cour.ttf",          # Courier New
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
    ]
    mono = None
    for p in mono_paths:
        if os.path.exists(p):
            try:
                mono = ImageFont.truetype(p, 36)
                break
            except:
                pass

    sans_paths = [
        "C:/Windows/Fonts/segoeui.ttf",       # Segoe UI
        "C:/Windows/Fonts/arial.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]
    sans = None
    for p in sans_paths:
        if os.path.exists(p):
            try:
                sans = ImageFont.truetype(p, 40)
                break
            except:
                pass

    bold_paths = [
        "C:/Windows/Fonts/segoeuib.ttf",      # Segoe UI Bold
        "C:/Windows/Fonts/arialbd.ttf",
        "/System/Library/Fonts/SFNSTextBold.ttf",
    ]
    sans_bold = None
    for p in bold_paths:
        if os.path.exists(p):
            try:
                sans_bold = ImageFont.truetype(p, 52)
                break
            except:
                pass

    if mono is None:
        mono = ImageFont.load_default()
    if sans is None:
        sans = mono
    if sans_bold is None:
        sans_bold = sans
    return sans, sans_bold, mono

SANS, SANS_BOLD, MONO = get_fonts()

def sized_font(paths, size):
    for p in paths:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except:
                pass
    return SANS

SANS_PATHS = [
    "C:/Windows/Fonts/segoeui.ttf",
    "C:/Windows/Fonts/arial.ttf",
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
]
MONO_PATHS = [
    "C:/Windows/Fonts/consola.ttf",
    "C:/Windows/Fonts/cour.ttf",
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
]

SANS_SMALL = sized_font(SANS_PATHS, 34)
SANS_LABEL = sized_font(SANS_PATHS, 40)
SANS_HEADING = sized_font(SANS_PATHS, 56)
MONO_LOG = sized_font(MONO_PATHS, 32)
SANS_TITLEBAR = sized_font(SANS_PATHS, 32)


def draw_windows_titlebar(draw, img, title="eudamed2swissdamed v0.1.4"):
    """Draw Windows 11-style light title bar with min/max/close buttons."""
    bar_h = 70
    # Title bar background
    draw.rectangle([0, 0, W, bar_h], fill=TITLE_BAR)
    draw.line([0, bar_h, W, bar_h], fill=TITLE_BAR_BORDER, width=1)

    # App icon (small)
    try:
        icon = Image.open(ICON_PATH).convert("RGBA")
        icon = icon.resize((36, 36), Image.LANCZOS)
        img.paste(icon, (20, 17), icon)
    except:
        pass

    # Title text
    draw.text((66, 19), title, fill=TEXT, font=SANS_TITLEBAR)

    # Window control buttons (minimize, maximize, close)
    btn_w = 70
    btn_h = bar_h
    x_start = W - 3 * btn_w

    # Minimize button (horizontal line)
    mx = x_start + btn_w // 2
    my = bar_h // 2
    draw.line([mx - 10, my, mx + 10, my], fill=(100, 100, 100), width=2)

    # Maximize button (square)
    mx2 = x_start + btn_w + btn_w // 2
    draw.rectangle([mx2 - 10, my - 10, mx2 + 10, my + 10], outline=(100, 100, 100), width=2)

    # Close button background + X
    close_x = x_start + 2 * btn_w
    draw.rectangle([close_x, 0, W, bar_h], fill=CLOSE_HOVER)
    cx = close_x + btn_w // 2
    cy = bar_h // 2
    draw.line([cx - 10, cy - 10, cx + 10, cy + 10], fill=(255, 255, 255), width=2)
    draw.line([cx - 10, cy + 10, cx + 10, cy - 10], fill=(255, 255, 255), width=2)


def draw_rounded_rect(draw, xy, radius, fill=None, outline=None):
    draw.rounded_rectangle(xy, radius=radius, fill=fill, outline=outline)


def draw_text_input(draw, x, y, w, h, text="", hint="", font=None):
    if font is None:
        font = SANS_LABEL
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=8, fill=WIDGET_BG, outline=BORDER)
    if text:
        draw.text((x+16, y+10), text, fill=TEXT, font=font)
    elif hint:
        draw.text((x+16, y+10), hint, fill=TEXT_DIM, font=font)


def draw_multiline_input(draw, x, y, w, h, lines=None, hint_lines=None, font=None):
    if font is None:
        font = SANS_LABEL
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=8, fill=WIDGET_BG, outline=BORDER)
    if lines:
        for i, line in enumerate(lines):
            draw.text((x+16, y+14+i*48), line, fill=TEXT, font=font)
    elif hint_lines:
        for i, line in enumerate(hint_lines):
            draw.text((x+16, y+14+i*48), line, fill=TEXT_DIM, font=font)


def draw_button(draw, x, y, w, h, text, enabled=True, accent=False):
    bg = BUTTON_BG if enabled else BUTTON_BG_DISABLED
    fg = BUTTON_TEXT if enabled else BUTTON_TEXT_DISABLED
    border = ACCENT if enabled else BORDER
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=8, fill=bg, outline=border)
    bbox = draw.textbbox((0, 0), text, font=SANS_LABEL)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    draw.text((x+(w-tw)//2, y+(h-th)//2 - 2), text, fill=fg, font=SANS_LABEL)


def draw_checkbox(draw, x, y, checked=False, label=""):
    s = 32
    draw_rounded_rect(draw, [x, y, x+s, y+s], radius=6,
                       fill=CHECKBOX_ON if checked else WIDGET_BG, outline=BORDER)
    if checked:
        draw.line([x+7, y+16, x+13, y+24], fill=(255, 255, 255), width=3)
        draw.line([x+13, y+24, x+25, y+8], fill=(255, 255, 255), width=3)
    draw.text((x+s+14, y-2), label, fill=TEXT, font=SANS_LABEL)


def draw_log_area(draw, x, y, w, h, lines, font=None):
    if font is None:
        font = MONO_LOG
    draw_rounded_rect(draw, [x, y, x+w, y+h], radius=8, fill=LOG_BG, outline=BORDER)
    clip_y = y + 14
    for line in lines:
        if clip_y + 38 > y + h - 14:
            break
        color = TEXT
        if line.startswith("=== DONE"):
            color = GREEN
        elif line.startswith("=== FAILED"):
            color = (220, 40, 40)
        elif line.startswith("["):
            color = ACCENT
        draw.text((x+18, clip_y), line, fill=color, font=font)
        clip_y += 40


def add_icon(img, x, y, size=100):
    try:
        icon = Image.open(ICON_PATH).convert("RGBA")
        icon = icon.resize((size, size), Image.LANCZOS)
        img.paste(icon, (x, y), icon)
    except:
        pass


def screenshot_1_main():
    """Screenshot 1: Main window, empty state with hint text."""
    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)
    draw_windows_titlebar(draw, img)

    margin = 80
    y = 110

    add_icon(img, W - margin - 100, y + 4, 100)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 90

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 58
    draw_multiline_input(draw, margin, y, W - 2*margin, 170,
                         hint_lines=["CH-MF-000023141", "CH-MF-000012345"])
    y += 196

    draw.text((margin, y+6), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+300, y, 180, 56, hint="all")
    draw_checkbox(draw, margin+540, y+14, checked=False, label="Dry run (no push)")
    y += 84

    draw.text((margin, y), "Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 66

    draw_button(draw, margin, y, 520, 68, "Download & Push", enabled=False, accent=True)
    y += 100

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 22

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 54

    draw_log_area(draw, margin, y, W-2*margin, H-y-60, [])

    img.save(os.path.join(OUT_DIR, "screenshot_1_main.png"))
    print("Saved screenshot_1_main.png")


def screenshot_2_running():
    """Screenshot 2: Download running with log output."""
    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)
    draw_windows_titlebar(draw, img)

    margin = 80
    y = 110

    add_icon(img, W - margin - 100, y + 4, 100)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 90

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 58
    draw_multiline_input(draw, margin, y, W - 2*margin, 170,
                         lines=["CH-MF-000023141"])
    y += 196

    draw.text((margin, y+6), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+300, y, 180, 56, text="50")
    draw_checkbox(draw, margin+540, y+14, checked=False, label="Dry run (no push)")
    y += 84

    draw.text((margin, y), "Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 66

    draw_button(draw, margin, y, 520, 68, "Running...", enabled=False)
    y += 100

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 22

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 54

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
        "[Download] Downloading detail 6/50: cb744f68-5ea4-48d3-...",
        "[Download] Downloading detail 7/50: 6e3662db-ecc9-43d1-...",
        "[Download] Downloading detail 8/50: e4a1b3c5-9f87-4321-...",
        "[Download] Downloading Basic UDI-DI data...",
        "[Download] Downloading basic UDI-DI 1/50...",
        "[Download] Downloading basic UDI-DI 2/50...",
        "[Download] Downloading basic UDI-DI 3/50...",
    ]
    draw_log_area(draw, margin, y, W-2*margin, H-y-60, log_lines)

    img.save(os.path.join(OUT_DIR, "screenshot_2_running.png"))
    print("Saved screenshot_2_running.png")


def screenshot_3_done():
    """Screenshot 3: Completed pipeline with success summary."""
    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)
    draw_windows_titlebar(draw, img)

    margin = 80
    y = 110

    add_icon(img, W - margin - 100, y + 4, 100)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 90

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 58
    draw_multiline_input(draw, margin, y, W - 2*margin, 170,
                         lines=["CH-MF-000023141"])
    y += 196

    draw.text((margin, y+6), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+300, y, 180, 56, text="50")
    draw_checkbox(draw, margin+540, y+14, checked=False, label="Dry run (no push)")
    y += 84

    draw.text((margin, y), "Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 66

    draw_button(draw, margin, y, 520, 68, "Download & Push", enabled=True, accent=True)
    y += 100

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 22

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 54

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
        "[Push] Submitting MDR device 1/50: 4f1e3733-2987-4d3b-...",
        "[Push] Submitting MDR device 2/50: 7cd1d81c-b335-4f95-...",
        "[Push] ...",
        "[Push] Submitted: 48, Failed: 0, Skipped: 2",
        "",
        "=== DONE === 48 submitted, 0 failed, 2 skipped",
    ]
    draw_log_area(draw, margin, y, W-2*margin, H-y-60, log_lines)

    img.save(os.path.join(OUT_DIR, "screenshot_3_done.png"))
    print("Saved screenshot_3_done.png")


def screenshot_4_credentials():
    """Screenshot 4: Swissdamed credentials expanded."""
    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)
    draw_windows_titlebar(draw, img)

    margin = 80
    y = 110

    add_icon(img, W - margin - 100, y + 4, 100)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 90

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 58
    draw_multiline_input(draw, margin, y, W - 2*margin, 170,
                         lines=["CH-MF-000023141", "DE-MF-000017808"])
    y += 196

    draw.text((margin, y+6), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+300, y, 180, 56, hint="all")
    draw_checkbox(draw, margin+540, y+14, checked=False, label="Dry run (no push)")
    y += 84

    draw.text((margin, y), "Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 58

    draw.text((margin+20, y+8), "Client ID:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+270, y, 660, 56, text="my-client-id-xxxxx")
    y += 74

    draw.text((margin+20, y+8), "Client Secret:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+270, y, 660, 56, text="****************")
    y += 74

    draw.text((margin+20, y+8), "API Base URL:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+270, y, 660, 56, text="https://playground.swissdamed.ch")
    y += 92

    draw_button(draw, margin, y, 520, 68, "Download & Push", enabled=True, accent=True)
    y += 100

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 22

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 54

    draw_log_area(draw, margin, y, W-2*margin, H-y-60, [])

    img.save(os.path.join(OUT_DIR, "screenshot_4_credentials.png"))
    print("Saved screenshot_4_credentials.png")


def screenshot_5_dryrun():
    """Screenshot 5: Dry run completed (download & preview only)."""
    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)
    draw_windows_titlebar(draw, img)

    margin = 80
    y = 110

    add_icon(img, W - margin - 100, y + 4, 100)
    draw.text((margin, y), "eudamed2swissdamed", fill=TEXT, font=SANS_HEADING)
    y += 90

    draw.text((margin, y), "SRNs (one per line or space-separated):", fill=TEXT, font=SANS_LABEL)
    y += 58
    draw_multiline_input(draw, margin, y, W - 2*margin, 170,
                         lines=["CH-MF-000023141"])
    y += 196

    draw.text((margin, y+6), "Limit per SRN:", fill=TEXT, font=SANS_LABEL)
    draw_text_input(draw, margin+300, y, 180, 56, text="10")
    draw_checkbox(draw, margin+540, y+14, checked=True, label="Dry run (no push)")
    y += 84

    draw.text((margin, y), "Swissdamed Credentials", fill=ACCENT, font=SANS_LABEL)
    y += 66

    draw_button(draw, margin, y, 520, 68, "Download & Preview", enabled=True, accent=True)
    y += 100

    draw.line([margin, y, W-margin, y], fill=SEPARATOR, width=2)
    y += 22

    draw.text((margin, y), "Log:", fill=TEXT, font=SANS_LABEL)
    y += 54

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
    draw_log_area(draw, margin, y, W-2*margin, H-y-60, log_lines)

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
    print("Size: 3840x2160 (4K) — PNG format")
