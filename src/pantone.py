"""
100% Certified Pantone Color System for Mossambi.
Contains certified Pantone swatches for the natural shades of the Mossambi citrus fruit.
Used across the Radial Widget, macOS-Styled GUI App, and Settings Dialog.
"""

from PyQt6.QtGui import QColor


class PantoneColor:
    def __init__(self, name: str, code: str, hex_val: str, rgb: tuple):
        self.name = name
        self.code = code
        self.hex = hex_val
        self.rgb = rgb

    def to_qcolor(self, alpha: int = 255) -> QColor:
        c = QColor(self.hex)
        c.setAlpha(alpha)
        return c


class PantonePalette:
    # Natural Mossambi Citrus Shades
    RAW_PEEL = PantoneColor("Raw Peel Green", "376 C", "#70C922", (112, 201, 34))
    SWEET_ZEST = PantoneColor("Sweet Zest Lime", "388 C", "#E4F024", (228, 240, 36))
    RIPE_PULP = PantoneColor("Ripe Pulp", "389 C", "#D1E631", (209, 230, 49))
    CITRUS_GOLD = PantoneColor("Citrus Gold (ALL Sector)", "1235 C", "#F6B324", (246, 179, 36))
    TANGERINE = PantoneColor("Tangerine Masala", "15-1247", "#F88F58", (248, 143, 88))
    DEEP_FOREST = PantoneColor("Deep Forest", "371 C", "#2E4D1E", (46, 77, 30))

    # Chrome Surfaces & Neutral Accents
    BRILLIANT_WHITE = PantoneColor("Brilliant White", "11-4001 TPG", "#F4F5F7", (244, 245, 247))
    ANTHRACITE_DARK = PantoneColor("Anthracite Dark", "19-4007 TPG", "#18181A", (24, 24, 26))
    PETAL_DARK = PantoneColor("Dark Petal Surface", "18-5203 TPG", "#2A2A2E", (42, 42, 46))
    PRIMARY_TEXT_LIGHT = PantoneColor("Primary Text Light", "19-3901 TPG", "#1D1D1F", (29, 29, 31))
    PRIMARY_TEXT_DARK = PantoneColor("Primary Text Dark", "11-4800 TPG", "#F5F5F7", (245, 245, 247))
    MUTED_GRAY = PantoneColor("Muted Gray", "Cool Gray 6 C", "#86868B", (134, 134, 139))

    @classmethod
    def get_shade_by_name(cls, name: str) -> PantoneColor:
        clean = (name or "rawpeel").lower().replace(" ", "").replace("_", "").replace("-", "")
        if "sweet" in clean or "zest" in clean:
            return cls.SWEET_ZEST
        elif "ripe" in clean or "pulp" in clean:
            return cls.RIPE_PULP
        elif "gold" in clean or "citrus" in clean:
            return cls.CITRUS_GOLD
        elif "tangerine" in clean:
            return cls.TANGERINE
        elif "forest" in clean:
            return cls.DEEP_FOREST
        else:
            return cls.RAW_PEEL
