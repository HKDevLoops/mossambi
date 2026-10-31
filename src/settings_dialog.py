"""
Mossambi Settings Window.
Implements the 4 sections from Design.md section 6:
General, Conversion, Appearance, About.
"""

import os
import json
from pathlib import Path
from typing import Dict, Any

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtWidgets import (
    QDialog, QVBoxLayout, QHBoxLayout, QLabel, QPushButton,
    QTabWidget, QWidget, QCheckBox, QComboBox, QFileDialog,
    QSlider, QGroupBox, QLineEdit
)

from .theme import Theme, ThemeMode, get_ui_font

CONFIG_DIR = Path.home() / ".mossambi"
CONFIG_FILE = CONFIG_DIR / "config.json"

DEFAULT_CONFIG: Dict[str, Any] = {
    "theme_mode": "system",
    "wheel_size": 440,
    "always_on_top": True,
    "play_sound": True,
    "launch_at_login": False,
    "output_mode": "sibling",  # "sibling" or "custom"
    "custom_output_dir": "",
    "quality": 92,
    "window_x": -1,
    "window_y": -1,
}


def load_config() -> Dict[str, Any]:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    if not CONFIG_FILE.exists():
        save_config(DEFAULT_CONFIG)
        return dict(DEFAULT_CONFIG)
    try:
        with open(CONFIG_FILE, "r", encoding="utf-8") as f:
            data = json.load(f)
            merged = dict(DEFAULT_CONFIG)
            merged.update(data)
            return merged
    except Exception:
        return dict(DEFAULT_CONFIG)


def save_config(config: Dict[str, Any]) -> None:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    try:
        with open(CONFIG_FILE, "w", encoding="utf-8") as f:
            json.dump(config, f, indent=2)
    except Exception as e:
        print(f"Error saving config: {e}")


class SettingsDialog(QDialog):
    """Clean, Apple-inspired settings modal window."""

    settings_changed = pyqtSignal(dict)

    def __init__(self, theme: Theme, parent=None):
        super().__init__(parent)
        self.theme = theme
        self.config = load_config()

        self.setWindowTitle("Mossambi — Settings")
        self.setFixedSize(520, 460)
        self.setWindowFlags(self.windowFlags() & ~Qt.WindowType.WindowContextHelpButtonHint)

        self._init_ui()
        self._apply_theme_styling()

    def _init_ui(self):
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(20, 20, 20, 20)
        main_layout.setSpacing(16)

        # Tabs
        self.tabs = QTabWidget()
        self.tabs.setFont(get_ui_font(size=12))

        # 1. General Tab
        tab_general = QWidget()
        gen_layout = QVBoxLayout(tab_general)
        gen_layout.setContentsMargins(16, 16, 16, 16)
        gen_layout.setSpacing(12)

        self.cb_always_on_top = QCheckBox("Keep widget always on top")
        self.cb_always_on_top.setChecked(self.config.get("always_on_top", True))

        self.cb_sound = QCheckBox("Play notification sound on completion")
        self.cb_sound.setChecked(self.config.get("play_sound", True))

        self.cb_launch_login = QCheckBox("Launch Mossambi when logging into Windows")
        self.cb_launch_login.setChecked(self.config.get("launch_at_login", False))

        hint_label = QLabel("Tip: Drag any file from Explorer directly onto the widget to convert.")
        hint_label.setWordWrap(True)
        hint_label.setStyleSheet("color: #86868B; font-size: 11px;")

        gen_layout.addWidget(self.cb_always_on_top)
        gen_layout.addWidget(self.cb_sound)
        gen_layout.addWidget(self.cb_launch_login)
        gen_layout.addSpacing(10)
        gen_layout.addWidget(hint_label)
        gen_layout.addStretch()

        # 2. Conversion Tab
        tab_conversion = QWidget()
        conv_layout = QVBoxLayout(tab_conversion)
        conv_layout.setContentsMargins(16, 16, 16, 16)
        conv_layout.setSpacing(14)

        # Output folder options
        out_group = QGroupBox("Destination Folder")
        out_group_layout = QVBoxLayout(out_group)

        self.combo_dest = QComboBox()
        self.combo_dest.addItems(["Same folder as original file (Sibling output)", "Custom folder..."])
        if self.config.get("output_mode") == "custom":
            self.combo_dest.setCurrentIndex(1)
        else:
            self.combo_dest.setCurrentIndex(0)
        self.combo_dest.currentIndexChanged.connect(self._toggle_dest_dir_row)

        dir_row = QHBoxLayout()
        self.txt_dest_dir = QLineEdit(self.config.get("custom_output_dir", ""))
        self.txt_dest_dir.setPlaceholderText("Select destination directory...")
        self.btn_browse = QPushButton("Browse...")
        self.btn_browse.clicked.connect(self._browse_custom_dir)
        dir_row.addWidget(self.txt_dest_dir)
        dir_row.addWidget(self.btn_browse)

        out_group_layout.addWidget(self.combo_dest)
        out_group_layout.addLayout(dir_row)
        conv_layout.addWidget(out_group)

        # Image Quality
        qual_group = QGroupBox("Lossy Image Quality (JPEG / WebP)")
        qual_layout = QVBoxLayout(qual_group)
        self.slider_quality = QSlider(Qt.Orientation.Horizontal)
        self.slider_quality.setRange(50, 100)
        self.slider_quality.setValue(self.config.get("quality", 92))
        self.lbl_qual_val = QLabel(f"Quality: {self.slider_quality.value()}%")
        self.slider_quality.valueChanged.connect(lambda v: self.lbl_qual_val.setText(f"Quality: {v}%"))
        qual_layout.addWidget(self.lbl_qual_val)
        qual_layout.addWidget(self.slider_quality)
        conv_layout.addWidget(qual_group)
        conv_layout.addStretch()

        # 3. Appearance Tab
        tab_appearance = QWidget()
        app_layout = QVBoxLayout(tab_appearance)
        app_layout.setContentsMargins(16, 16, 16, 16)
        app_layout.setSpacing(14)

        # Theme selector
        theme_row = QHBoxLayout()
        theme_label = QLabel("Color Theme:")
        self.combo_theme = QComboBox()
        self.combo_theme.addItems(["System Default", "Light Theme", "Dark Theme"])
        mode_val = self.config.get("theme_mode", "system")
        if mode_val == "light":
            self.combo_theme.setCurrentIndex(1)
        elif mode_val == "dark":
            self.combo_theme.setCurrentIndex(2)
        else:
            self.combo_theme.setCurrentIndex(0)
        theme_row.addWidget(theme_label)
        theme_row.addWidget(self.combo_theme)
        app_layout.addLayout(theme_row)

        # Size selector
        size_row = QHBoxLayout()
        size_label = QLabel("Widget Size:")
        self.combo_size = QComboBox()
        self.combo_size.addItems(["Small (380 px)", "Medium (440 px)", "Large (500 px)"])
        sz_val = self.config.get("wheel_size", 440)
        if sz_val <= 380:
            self.combo_size.setCurrentIndex(0)
        elif sz_val >= 500:
            self.combo_size.setCurrentIndex(2)
        else:
            self.combo_size.setCurrentIndex(1)
        size_row.addWidget(size_label)
        size_row.addWidget(self.combo_size)
        app_layout.addLayout(size_row)
        app_layout.addStretch()

        # 4. About Tab
        tab_about = QWidget()
        abt_layout = QVBoxLayout(tab_about)
        abt_layout.setContentsMargins(16, 16, 16, 16)
        abt_layout.setSpacing(10)

        app_title = QLabel("Mossambi")
        app_title.setFont(get_ui_font(size=18, bold=True))
        app_subtitle = QLabel("Native Offline Desktop File Converter")
        app_subtitle.setStyleSheet("color: #86868B; font-size: 12px;")

        privacy_box = QLabel(
            "Privacy Guarantee:\n"
            "Mossambi runs 100% locally on your device. Your files are never uploaded to any server, "
            "and no analytics, telemetry, or personal data are collected."
        )
        privacy_box.setWordWrap(True)
        privacy_box.setStyleSheet(
            "background: rgba(120, 169, 66, 0.12); padding: 12px; border-radius: 8px; color: #437119; font-size: 11px;"
        )

        version_lbl = QLabel("Version: 1.0.0 (Windows Native)")
        version_lbl.setStyleSheet("color: #86868B; font-size: 11px;")

        abt_layout.addWidget(app_title)
        abt_layout.addWidget(app_subtitle)
        abt_layout.addSpacing(10)
        abt_layout.addWidget(privacy_box)
        abt_layout.addSpacing(10)
        abt_layout.addWidget(version_lbl)
        abt_layout.addStretch()

        # Add all tabs
        self.tabs.addTab(tab_general, "General")
        self.tabs.addTab(tab_conversion, "Conversion")
        self.tabs.addTab(tab_appearance, "Appearance")
        self.tabs.addTab(tab_about, "About")

        main_layout.addWidget(self.tabs)

        # Buttons
        btn_box = QHBoxLayout()
        btn_box.addStretch()
        self.btn_cancel = QPushButton("Cancel")
        self.btn_cancel.clicked.connect(self.reject)
        self.btn_save = QPushButton("Save Settings")
        self.btn_save.setDefault(True)
        self.btn_save.setStyleSheet(
            "background-color: #78A942; color: white; font-weight: bold; padding: 6px 16px; border-radius: 6px;"
        )
        self.btn_save.clicked.connect(self._save_and_close)
        btn_box.addWidget(self.btn_cancel)
        btn_box.addWidget(self.btn_save)
        main_layout.addLayout(btn_box)

        self._toggle_dest_dir_row(self.combo_dest.currentIndex())

    def _toggle_dest_dir_row(self, index: int):
        is_custom = (index == 1)
        self.txt_dest_dir.setEnabled(is_custom)
        self.btn_browse.setEnabled(is_custom)

    def _browse_custom_dir(self):
        folder = QFileDialog.getExistingDirectory(self, "Select Output Folder")
        if folder:
            self.txt_dest_dir.setText(folder)

    def _apply_theme_styling(self):
        is_dark = self.theme.is_dark
        bg_col = "#1E1E22" if is_dark else "#F5F5F7"
        text_col = "#F5F5F7" if is_dark else "#1D1D1F"
        tab_bg = "#27272A" if is_dark else "#FFFFFF"

        self.setStyleSheet(f"""
            QDialog {{
                background-color: {bg_col};
                color: {text_col};
            }}
            QTabWidget::pane {{
                background: {tab_bg};
                border: 1px solid {'#3F3F46' if is_dark else '#E5E5EA'};
                border-radius: 8px;
            }}
            QLabel, QCheckBox {{
                color: {text_col};
            }}
            QGroupBox {{
                color: {text_col};
                font-weight: 500;
                border: 1px solid {'#3F3F46' if is_dark else '#E5E5EA'};
                border-radius: 6px;
                margin-top: 10px;
                padding-top: 10px;
            }}
            QGroupBox::title {{
                subcontrol-origin: margin;
                left: 10px;
                padding: 0 4px;
            }}
        """)

    def _save_and_close(self):
        # Update config dictionary
        self.config["always_on_top"] = self.cb_always_on_top.isChecked()
        self.config["play_sound"] = self.cb_sound.isChecked()
        self.config["launch_at_login"] = self.cb_launch_login.isChecked()

        dest_idx = self.combo_dest.currentIndex()
        self.config["output_mode"] = "custom" if dest_idx == 1 else "sibling"
        self.config["custom_output_dir"] = self.txt_dest_dir.text().strip()

        self.config["quality"] = self.slider_quality.value()

        theme_idx = self.combo_theme.currentIndex()
        theme_modes = ["system", "light", "dark"]
        self.config["theme_mode"] = theme_modes[theme_idx]

        size_idx = self.combo_size.currentIndex()
        sizes = [380, 440, 500]
        self.config["wheel_size"] = sizes[size_idx]

        save_config(self.config)
        self.settings_changed.emit(self.config)
        self.accept()
