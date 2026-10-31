"""
Main Mossambi Desktop Widget Window.
Frameless, translucent, always-on-top desktop widget with native drag-and-drop.
"""

import os
import sys
import subprocess
from typing import List, Optional

from PyQt6.QtCore import Qt, QPoint, QPointF
from PyQt6.QtGui import QDragEnterEvent, QDragMoveEvent, QDragLeaveEvent, QDropEvent, QKeyEvent
from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QFileDialog, QMenu
)

from .theme import Theme, ThemeMode
from .wheel import RadialWheel, WheelState
from .registry import get_available_formats, DEFAULT_IDLE_SECTORS
from .converter import ConversionWorker
from .settings_dialog import SettingsDialog, load_config, save_config


class MossambiWidget(QWidget):
    """
    Floating, frameless, translucent desktop widget for zero-click file conversion.
    """

    def __init__(self):
        super().__init__()
        self.config = load_config()

        # Theme initialization
        theme_mode_str = self.config.get("theme_mode", "system")
        try:
            mode = ThemeMode(theme_mode_str)
        except ValueError:
            mode = ThemeMode.SYSTEM
        self.theme = Theme(mode)

        # Active worker thread reference
        self.active_worker: Optional[ConversionWorker] = None

        self._init_window()
        self._init_ui()
        self._restore_position()

    def _init_window(self):
        self.setWindowTitle("Mossambi")

        # Frameless and translucent window flags
        flags = Qt.WindowType.FramelessWindowHint | Qt.WindowType.SubWindow
        if self.config.get("always_on_top", True):
            flags |= Qt.WindowType.WindowStaysOnTopHint
        self.setWindowFlags(flags)

        # Translucent background attribute for 32-bit alpha rendering
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground, True)
        self.setAttribute(Qt.WidgetAttribute.WA_NoSystemBackground, True)

        # Native drag-and-drop
        self.setAcceptDrops(True)

        # Set size based on configuration
        size = self.config.get("wheel_size", 440)
        self.resize(size, size)

    def _init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)

        self.wheel = RadialWheel(self.theme, self)
        layout.addWidget(self.wheel)

        # Connect wheel signals
        self.wheel.sector_selected.connect(self._on_sector_selected)
        self.wheel.center_clicked.connect(self._on_center_clicked)
        self.wheel.widget_dragged.connect(self._on_widget_dragged)

    def _restore_position(self):
        x = self.config.get("window_x", -1)
        y = self.config.get("window_y", -1)

        screen = self.screen()
        if screen:
            screen_geo = screen.availableGeometry()
            w, h = self.width(), self.height()

            if x >= 0 and y >= 0 and screen_geo.contains(x, y):
                self.move(x, y)
            else:
                # Center on screen by default
                cx = screen_geo.x() + (screen_geo.width() - w) // 2
                cy = screen_geo.y() + (screen_geo.height() - h) // 2
                self.move(cx, cy)

    def _save_position(self):
        pos = self.pos()
        self.config["window_x"] = pos.x()
        self.config["window_y"] = pos.y()
        save_config(self.config)

    def set_theme_mode(self, mode: ThemeMode):
        self.theme = Theme(mode)
        self.wheel.set_theme(self.theme)
        self.config["theme_mode"] = mode.value
        save_config(self.config)

    def set_wheel_size(self, size: int):
        self.config["wheel_size"] = size
        save_config(self.config)
        self.resize(size, size)
        self.wheel.update()

    def set_always_on_top(self, enable: bool):
        self.config["always_on_top"] = enable
        save_config(self.config)

        # Re-apply window flags
        pos = self.pos()
        flags = Qt.WindowType.FramelessWindowHint | Qt.WindowType.SubWindow
        if enable:
            flags |= Qt.WindowType.WindowStaysOnTopHint
        self.setWindowFlags(flags)
        self.show()
        self.move(pos)

    # Mouse & Window Dragging
    def _on_widget_dragged(self, delta: QPointF):
        new_pos = self.pos() + QPoint(int(delta.x()), int(delta.y()))
        self.move(new_pos)
        self._save_position()

    # User Interactions
    def _on_center_clicked(self):
        """Open native file picker when center hub is clicked."""
        files, _ = QFileDialog.getOpenFileNames(
            self,
            "Select files to convert",
            "",
            "All Files (*.*)"
        )
        if files:
            self._load_files(files)

    def _load_files(self, files: List[str]):
        """Analyze files and set appropriate sectors."""
        self.wheel.set_current_files(files)
        available = get_available_formats(files)
        self.wheel.set_sectors(available)

    def _on_sector_selected(self, target_format: str):
        """Sector clicked or selected."""
        if not self.wheel.current_files:
            # If no file loaded yet, ask user to choose a file for this format
            files, _ = QFileDialog.getOpenFileNames(
                self,
                f"Choose files to convert to {target_format}",
                "",
                "All Files (*.*)"
            )
            if files:
                self.wheel.set_current_files(files)
                self._execute_conversion(files, target_format)
        else:
            self._execute_conversion(self.wheel.current_files, target_format)

    def _execute_conversion(self, files: List[str], target_format: str):
        """Launch background conversion worker."""
        if self.active_worker and self.active_worker.isRunning():
            return  # Conversion already in progress

        self.wheel.start_conversion(target_format)

        output_dir = None
        if self.config.get("output_mode") == "custom":
            output_dir = self.config.get("custom_output_dir") or None

        quality = self.config.get("quality", 92)

        self.active_worker = ConversionWorker(
            filepaths=files,
            target_format=target_format,
            output_dir=output_dir,
            quality=quality
        )
        self.active_worker.progress_changed.connect(self.wheel.update_progress)
        self.active_worker.conversion_finished.connect(self._on_conversion_finished)
        self.active_worker.start()

    def _on_conversion_finished(self, success: bool, output_path: str, error_msg: str):
        if success:
            self.wheel.set_success(output_path)
            if self.config.get("play_sound", True):
                try:
                    import winsound
                    winsound.MessageBeep(winsound.MB_OK)
                except Exception:
                    pass
        else:
            self.wheel.set_error(error_msg)

    # Native OLE Drag & Drop Implementation
    def dragEnterEvent(self, event: QDragEnterEvent):
        if event.mimeData().hasUrls():
            paths = [u.toLocalFile() for u in event.mimeData().urls() if u.toLocalFile()]
            if paths:
                event.acceptProposedAction()
                self._load_files(paths)
                self.wheel.state = WheelState.FILE_HOVER
                self.wheel.update()
                return
        event.ignore()

    def dragMoveEvent(self, event: QDragMoveEvent):
        if event.mimeData().hasUrls():
            event.acceptProposedAction()
            # Forward coordinate to wheel hit testing
            local_pos = self.wheel.mapFrom(self, event.position().toPoint())
            idx, is_hub = self.wheel._hit_test(QPointF(local_pos.x(), local_pos.y()))
            if self.wheel.hovered_index != idx or self.wheel.is_center_hovered != is_hub:
                self.wheel.hovered_index = idx
                self.wheel.is_center_hovered = is_hub
                self.wheel.update()

    def dragLeaveEvent(self, event: QDragLeaveEvent):
        if self.wheel.state != WheelState.CONVERTING:
            self.wheel.hovered_index = None
            self.wheel.is_center_hovered = False
            self.wheel.update()

    def dropEvent(self, event: QDropEvent):
        if not event.mimeData().hasUrls():
            event.ignore()
            return

        paths = [u.toLocalFile() for u in event.mimeData().urls() if u.toLocalFile()]
        if not paths:
            event.ignore()
            return

        event.acceptProposedAction()

        # Check if dropped onto a specific sector
        local_pos = self.wheel.mapFrom(self, event.position().toPoint())
        idx, is_hub = self.wheel._hit_test(QPointF(local_pos.x(), local_pos.y()))

        if idx is not None and 0 <= idx < len(self.wheel.sectors):
            target_fmt = self.wheel.sectors[idx]
            self.wheel.set_current_files(paths)
            self._execute_conversion(paths, target_fmt)
        else:
            # Dropped onto center hub: load file and keep wheel open to pick format
            self._load_files(paths)

    # Keyboard Navigation
    def keyPressEvent(self, event: QKeyEvent):
        key = event.key()
        n = len(self.wheel.sectors)

        if key in (Qt.Key.Key_Right, Qt.Key.Key_Down):
            # Next sector
            current = self.wheel.hovered_index if self.wheel.hovered_index is not None else -1
            self.wheel.hovered_index = (current + 1) % n
            self.wheel.update()
        elif key in (Qt.Key.Key_Left, Qt.Key.Key_Up):
            # Previous sector
            current = self.wheel.hovered_index if self.wheel.hovered_index is not None else 0
            self.wheel.hovered_index = (current - 1 + n) % n
            self.wheel.update()
        elif key in (Qt.Key.Key_Return, Qt.Key.Key_Enter, Qt.Key.Key_Space):
            # Commit selected sector
            if self.wheel.hovered_index is not None and 0 <= self.wheel.hovered_index < n:
                fmt = self.wheel.sectors[self.wheel.hovered_index]
                self._on_sector_selected(fmt)
        elif key == Qt.Key.Key_Escape:
            self.wheel.reset_to_idle()
            self.wheel.set_current_files([])
            self.wheel.set_sectors(DEFAULT_IDLE_SECTORS)
        else:
            super().keyPressEvent(event)

    # Right-Click Context Menu
    def contextMenuEvent(self, event):
        menu = QMenu(self)
        menu.setStyleSheet("""
            QMenu {
                background-color: #242428;
                color: #F5F5F7;
                border: 1px solid #3F3F46;
                border-radius: 8px;
                padding: 6px;
            }
            QMenu::item {
                padding: 6px 20px;
                border-radius: 4px;
            }
            QMenu::item:selected {
                background-color: #78A942;
                color: white;
            }
            QMenu::separator {
                height: 1px;
                background: #3F3F46;
                margin: 4px 8px;
            }
        """)

        act_file = menu.addAction("Choose File...")
        act_file.triggered.connect(self._on_center_clicked)

        # Open Output Folder
        act_reveal = menu.addAction("Open Output Folder")
        act_reveal.triggered.connect(self._reveal_output_folder)

        menu.addSeparator()

        # Appearance submenu
        menu_theme = menu.addMenu("Theme")
        act_light = menu_theme.addAction("Light")
        act_dark = menu_theme.addAction("Dark")
        act_sys = menu_theme.addAction("System Default")
        act_light.triggered.connect(lambda: self.set_theme_mode(ThemeMode.LIGHT))
        act_dark.triggered.connect(lambda: self.set_theme_mode(ThemeMode.DARK))
        act_sys.triggered.connect(lambda: self.set_theme_mode(ThemeMode.SYSTEM))

        # Size submenu
        menu_size = menu.addMenu("Widget Size")
        act_sz_s = menu_size.addAction("Small (380 px)")
        act_sz_m = menu_size.addAction("Medium (440 px)")
        act_sz_l = menu_size.addAction("Large (500 px)")
        act_sz_s.triggered.connect(lambda: self.set_wheel_size(380))
        act_sz_m.triggered.connect(lambda: self.set_wheel_size(440))
        act_sz_l.triggered.connect(lambda: self.set_wheel_size(500))

        # Always on top toggle
        act_ontop = menu.addAction("Always On Top")
        act_ontop.setCheckable(True)
        act_ontop.setChecked(self.config.get("always_on_top", True))
        act_ontop.triggered.connect(lambda: self.set_always_on_top(act_ontop.isChecked()))

        # Recenter Widget
        act_center = menu.addAction("Reset Position to Center")
        act_center.triggered.connect(self._recenter_widget)

        menu.addSeparator()

        # Open Full macOS Desktop App
        act_desktop_app = menu.addAction("Open Full Desktop App...")
        act_desktop_app.triggered.connect(self._open_full_desktop_app)

        # Minimize to tray
        act_minimize = menu.addAction("Minimize to Tray")
        act_minimize.triggered.connect(self.hide)

        # Exit
        act_exit = menu.addAction("Quit Mossambi")
        act_exit.triggered.connect(sys.exit)

        menu.exec(event.globalPos())

    def _reveal_output_folder(self):
        target_dir = None
        if self.wheel.last_output_path and os.path.exists(self.wheel.last_output_path):
            target_dir = os.path.dirname(self.wheel.last_output_path)
        elif self.config.get("custom_output_dir") and os.path.exists(self.config["custom_output_dir"]):
            target_dir = self.config["custom_output_dir"]
        elif self.wheel.current_files and os.path.exists(self.wheel.current_files[0]):
            target_dir = os.path.dirname(self.wheel.current_files[0])
        else:
            target_dir = os.path.expanduser("~")

        if sys.platform == "win32":
            subprocess.run(["explorer.exe", target_dir])
        elif sys.platform == "darwin":
            subprocess.run(["open", target_dir])
        else:
            subprocess.run(["xdg-open", target_dir])

    def _recenter_widget(self):
        screen = self.screen()
        if screen:
            geo = screen.availableGeometry()
            cx = geo.x() + (geo.width() - self.width()) // 2
            cy = geo.y() + (geo.height() - self.height()) // 2
            self.move(cx, cy)
            self._save_position()

    def summon_at_cursor(self, cursor_x: int, cursor_y: int):
        half_w = self.width() // 2
        half_h = self.height() // 2
        new_x = max(0, cursor_x - half_w)
        new_y = max(0, cursor_y - half_h)
        self.move(new_x, new_y)
        self.show()
        self.raise_()
        self.activateWindow()

    def _open_full_desktop_app(self):
        from .full_app import MossambiDesktopApp
        if not hasattr(self, "_desktop_app") or self._desktop_app is None:
            self._desktop_app = MossambiDesktopApp(self.theme)
        self._desktop_app.show()
        self._desktop_app.raise_()
        self._desktop_app.activateWindow()

    def _open_settings(self):
        dlg = SettingsDialog(self.theme, self)
        dlg.settings_changed.connect(self._on_settings_updated)
        dlg.exec()

    def _on_settings_updated(self, new_config: dict):
        self.config = new_config
        # Update theme and Pantone citrus shade
        mode_str = new_config.get("theme_mode", "system")
        shade = new_config.get("citrus_shade", "rawpeel")
        try:
            self.theme = Theme(ThemeMode(mode_str), citrus_shade=shade)
        except ValueError:
            self.theme = Theme(ThemeMode.SYSTEM, citrus_shade=shade)
        self.wheel.set_theme(self.theme)

        # Update size
        sz = new_config.get("wheel_size", 440)
        self.resize(sz, sz)

        # Update on top
        ontop = new_config.get("always_on_top", True)
        self.set_always_on_top(ontop)
