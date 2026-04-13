from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
ICONS_DIR = ROOT / "src-tauri" / "icons"
ICONSET_DIR = ICONS_DIR / "icon.iconset"


def rounded_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def vertical_gradient(size: int, top: tuple[int, int, int], bottom: tuple[int, int, int]) -> Image.Image:
    base = Image.new("RGBA", (size, size))
    px = base.load()
    for y in range(size):
        t = y / max(size - 1, 1)
        color = tuple(int(top[i] * (1 - t) + bottom[i] * t) for i in range(3)) + (255,)
        for x in range(size):
            px[x, y] = color
    return base


def draw_background(size: int) -> Image.Image:
    bg = vertical_gradient(size, (232, 232, 234), (198, 198, 202))
    mask = rounded_mask(size, size // 4)
    tile = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    tile.paste(bg, (0, 0), mask)

    overlay = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    draw.rounded_rectangle(
        (size * 0.06, size * 0.06, size * 0.94, size * 0.94),
        radius=size * 0.19,
        outline=(255, 255, 255, 140),
        width=max(2, size // 96),
    )
    return Image.alpha_composite(tile, overlay)


def draw_mic(size: int) -> Image.Image:
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    center_x = size / 2
    capsule_w = size * 0.28
    capsule_h = size * 0.42
    capsule_left = center_x - capsule_w / 2
    capsule_top = size * 0.18
    capsule_box = (
        capsule_left,
        capsule_top,
        capsule_left + capsule_w,
        capsule_top + capsule_h,
    )

    body_fill = (20, 20, 22, 255)
    stroke = (0, 0, 0, 255)

    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(shadow)
    offset = size * 0.012
    sdraw.rounded_rectangle(
        (
            capsule_box[0] + offset,
            capsule_box[1] + offset,
            capsule_box[2] + offset,
            capsule_box[3] + offset,
        ),
        radius=capsule_w / 2,
        fill=(0, 0, 0, 72),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(size * 0.018))
    layer = Image.alpha_composite(layer, shadow)

    draw = ImageDraw.Draw(layer)
    draw.rounded_rectangle(capsule_box, radius=capsule_w / 2, fill=body_fill)
    draw.rounded_rectangle(capsule_box, radius=capsule_w / 2, outline=stroke, width=max(3, size // 110))

    grille_w = max(2, size // 44)
    for i in range(4):
        x = capsule_left + capsule_w * (0.2 + i * 0.2)
        draw.rounded_rectangle(
            (x, capsule_top + capsule_h * 0.16, x + grille_w, capsule_top + capsule_h * 0.84),
            radius=grille_w / 2,
            fill=(74, 74, 78, 255),
        )

    draw = ImageDraw.Draw(layer)
    yoke_w = max(6, size // 28)
    draw.arc(
        (
            center_x - capsule_w * 0.82,
            capsule_top + capsule_h * 0.44,
            center_x + capsule_w * 0.82,
            capsule_top + capsule_h * 1.22,
        ),
        start=24,
        end=156,
        fill=(18, 18, 20, 255),
        width=yoke_w,
    )

    stem_w = max(8, size // 22)
    stem_h = size * 0.12
    stem_box = (
        center_x - stem_w / 2,
        capsule_top + capsule_h * 1.0,
        center_x + stem_w / 2,
        capsule_top + capsule_h * 1.0 + stem_h,
    )
    draw.rounded_rectangle(stem_box, radius=stem_w / 2, fill=(18, 18, 20, 255))

    base_y = stem_box[3] + size * 0.02
    draw.rounded_rectangle(
        (
            center_x - size * 0.17,
            base_y,
            center_x + size * 0.17,
            base_y + size * 0.048,
        ),
        radius=size * 0.024,
        fill=(18, 18, 20, 255),
        outline=stroke,
        width=max(2, size // 140),
    )
    return layer


def render_master(size: int = 1024) -> Image.Image:
    bg = draw_background(size)
    mic = draw_mic(size)
    return Image.alpha_composite(bg, mic)


def save_png(img: Image.Image, path: Path, size: int) -> None:
    img.resize((size, size), Image.Resampling.LANCZOS).save(path)


def main() -> None:
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    ICONSET_DIR.mkdir(parents=True, exist_ok=True)

    master = render_master(1024)
    master.save(ICONS_DIR / "icon.png")

    save_png(master, ICONS_DIR / "32x32.png", 32)
    save_png(master, ICONS_DIR / "128x128.png", 128)
    save_png(master, ICONS_DIR / "128x128@2x.png", 256)

    extra_pngs = {
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
        "StoreLogo.png": 50,
    }
    for name, size in extra_pngs.items():
        save_png(master, ICONS_DIR / name, size)

    iconset_sizes = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }
    for name, size in iconset_sizes.items():
        save_png(master, ICONSET_DIR / name, size)

    master.save(
        ICONS_DIR / "icon.ico",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    master.save(ICONS_DIR / "icon.icns")


if __name__ == "__main__":
    main()
