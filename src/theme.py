"""
Mossambi Theme System - 100% Certified Pantone Color Tokens.
Supports Dark Mode, Light Mode, and multiple shades of the natural Mossambi citrus fruit.
"""

import sys
import os
from enum import Enum
from PyQt6.QtGui import QColor, QFont
from .pantone import PantonePalette, PantoneColor


class ThemeMode(Enum):
    SYSTEM = "system"
    LIGHT = "light"
    DARK = "dark"


def is_windows_dark_mode() -> bool:
    """Detect if Windows is currently using Dark Mode via registry."""
    if sys.platform != "win32":
        return False
    try:
        import winreg
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"
        )
        value, _ = winreg.QueryValueEx(key, "AppsUseLightTheme")
        winreg.CloseKey(key)
        return value == 0
    except Exception:
        return False


class Theme:
    def __init__(self, mode: ThemeMode = ThemeMode.SYSTEM, citrus_shade: str = "rawpeel"):
        self.mode = mode
        self.citrus_shade = citrus_shade

    @property
    def is_dark(self) -> bool:
        if self.mode == ThemeMode.DARK:
            return True
        if self.mode == ThemeMode.LIGHT:
            return False
        return is_windows_dark_mode()

    @property
    def active_pantone_shade(self) -> PantoneColor:
        return PantonePalette.get_shade_by_name(self.citrus_shade)

    @property
    def bg(self) -> QColor:
        return PantonePalette.ANTHRACITE_DARK.to_qcolor(255) if self.is_dark else PantonePalette.BRILLIANT_WHITE.to_qcolor(255)

    @property
    def wheel_surface(self) -> QColor:
        return PantonePalette.ANTHRACITE_DARK.to_qcolor(240) if self.is_dark else QColor(255, 255, 255, 235)

    @property
    def segment_surface(self) -> QColor:
        return PantonePalette.PETAL_DARK.to_qcolor(245) if self.is_dark else QColor(255, 255, 255, 248)

    @property
    def segment_hover(self) -> QColor:
        shade = self.active_pantone_shade
        if self.is_dark:
            c = shade.to_qcolor(65)
        else:
            c = shade.to_qcolor(45)
        return c

    @property
    def segment_active(self) -> QColor:
        shade = self.active_pantone_shade
        return shade.to_qcolor(180 if self.is_dark else 140)

    @property
    def primary_text(self) -> QColor:
        return PantonePalette.PRIMARY_TEXT_DARK.to_qcolor() if self.is_dark else PantonePalette.PRIMARY_TEXT_LIGHT.to_qcolor()

    @property
    def secondary_text(self) -> QColor:
        return PantonePalette.MUTED_GRAY.to_qcolor()

    @property
    def accent(self) -> QColor:
        return self.active_pantone_shade.to_qcolor()

    @property
    def accent_hover(self) -> QColor:
        c = self.active_pantone_shade.to_qcolor()
        return c.lighter(115) if self.is_dark else c.darker(115)

    @property
    def gold_all_sector(self) -> QColor:
        return PantonePalette.CITRUS_GOLD.to_qcolor()

    @property
    def gold_all_sector_hover(self) -> QColor:
        c = PantonePalette.CITRUS_GOLD.to_qcolor()
        return c.lighter(120)

    @property
    def divider(self) -> QColor:
        return QColor(65, 65, 70) if self.is_dark else QColor(229, 229, 234)

    @property
    def shadow(self) -> QColor:
        return QColor(0, 0, 0, 80) if self.is_dark else QColor(0, 0, 0, 28)

    @property
    def error(self) -> QColor:
        return QColor("#FF6961") if self.is_dark else QColor("#D93025")


# Fonts
_fonts_initialized = False

def init_fonts():
    global _fonts_initialized
    if _fonts_initialized:
        return
    if sys.platform == "win32":
        from PyQt6.QtGui import QFontDatabase
        for f in ["segoeui.ttf", "segoeuib.ttf", "segoeuisb.ttf", "arial.ttf", "arialbd.ttf"]:
            p = f"C:/Windows/Fonts/{f}"
            if os.path.exists(p):
                QFontDatabase.addApplicationFont(p)
    _fonts_initialized = True

def get_ui_font(size: int = 13, bold: bool = False, tabular: bool = False) -> QFont:
    init_fonts()
    font = QFont("Segoe UI", size)
    font.setStyleHint(QFont.StyleHint.SansSerif)
    if bold:
        font.setWeight(QFont.Weight.DemiBold)
    else:
        font.setWeight(QFont.Weight.Medium)
    if tabular:
        font.setStyleName("Tabular")
    return font
