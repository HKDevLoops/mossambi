"""
Mossambi Application Entry Point & Tray Icon Manager.
Sets up High-DPI scaling, Windows Process AppID, System Tray,
Win+Ctrl global hotkey, Circle-Drag gesture wakeup, and Main Loop.
"""

import sys
import ctypes
from PyQt6.QtCore import Qt
from PyQt6.QtGui import (
    QIcon, QPixmap, QPainter, QColor, QBrush, QPen, QAction
)
from PyQt6.QtWidgets import QApplication, QSystemTrayIcon, QMenu

from .widget import MossambiWidget
from .theme import ThemeMode
from .gesture import WakeupListener
from .pantone import PantonePalette


def create_tray_icon_pixmap(size: int = 64) -> QPixmap:
    """Renders a sleek vector Mossambi icon with Pantone 376 C Citrus Green for Taskbar & Tray."""
    pixmap = QPixmap(size, size)
    pixmap.fill(Qt.GlobalColor.transparent)

    p = QPainter(pixmap)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)

    cx = size / 2.0
    cy = size / 2.0
    r = size * 0.42

    # Outer mossambi raw peel green circular badge (Pantone 376 C)
    p.setBrush(QBrush(QColor(PantonePalette.RAW_PEEL.hex)))
    p.setPen(QPen(QColor(255, 255, 255, 70), 1.5))
    p.drawEllipse(int(cx - r), int(cy - r), int(r * 2), int(r * 2))

    # Inner circular hub
    inner_r = r * 0.45
    p.setBrush(QBrush(QColor(255, 255, 255, 240)))
    p.setPen(Qt.PenStyle.NoPen)
    p.drawEllipse(int(cx - inner_r), int(cy - inner_r), int(inner_r * 2), int(inner_r * 2))

    # Center dot in Sweet Zest Lime (Pantone 388 C)
    dot_r = inner_r * 0.4
    p.setBrush(QBrush(QColor(PantonePalette.SWEET_ZEST.hex)))
    p.drawEllipse(int(cx - dot_r), int(cy - dot_r), int(dot_r * 2), int(dot_r * 2))

    p.end()
    return pixmap


def run_app():
    # Set Windows App ID for high-fidelity taskbar grouping & notifications
    if sys.platform == "win32":
        try:
            myappid = "mossambi.desktop.suite.2.1"
            ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)
        except Exception:
            pass

    app = QApplication(sys.argv)
    app.setApplicationName("Mossambi")
    app.setApplicationDisplayName("Mossambi Desktop Suite")
    app.setQuitOnLastWindowClosed(False)

    # Set Application Icon
    icon_pixmap = create_tray_icon_pixmap(128)
    app_icon = QIcon(icon_pixmap)
    app.setWindowIcon(app_icon)

    # Create Main Desktop Radial Widget
    widget = MossambiWidget()
    widget.show()

    # Create Background Wakeup Listener (Win+Ctrl hotkey and circle-drag gesture)
    listener = WakeupListener()
    listener.summon_hotkey.connect(widget.summon_at_cursor)
    listener.summon_circle_gesture.connect(widget.summon_at_cursor)
    listener.start()
    app.aboutToQuit.connect(listener.stop)

    # Create System Tray Icon
    tray = QSystemTrayIcon(app_icon, app)
    tray.setToolTip("Mossambi — Desktop Zero-Loss Transcoder & Exam Tuner")

    tray_menu = QMenu()
    tray_menu.setStyleSheet("""
        QMenu {
            background-color: #242428;
            color: #F5F5F7;
            border: 1px solid #3F3F46;
            border-radius: 6px;
            padding: 4px;
        }
        QMenu::item {
            padding: 6px 18px;
            border-radius: 4px;
        }
        QMenu::item:selected {
            background-color: #70C922;
            color: white;
            font-weight: bold;
        }
    """)

    act_show = QAction("Show Radial Wheel", tray_menu)
    act_show.triggered.connect(lambda: (widget.show(), widget.raise_(), widget.activateWindow()))
    tray_menu.addAction(act_show)

    act_full_app = QAction("Open macOS Desktop App (Exam Studio & Catalog)...", tray_menu)
    act_full_app.triggered.connect(widget._open_full_desktop_app)
    tray_menu.addAction(act_full_app)

    act_hide = QAction("Hide Radial Wheel", tray_menu)
    act_hide.triggered.connect(widget.hide)
    tray_menu.addAction(act_hide)

    tray_menu.addSeparator()

    # Settings Dialog access ONLY through System Tray (and TUI / CLI)
    act_settings = QAction("Settings & Pantone Customizer...", tray_menu)
    act_settings.triggered.connect(widget._open_settings)
    tray_menu.addAction(act_settings)

    act_quit = QAction("Quit Mossambi", tray_menu)
    act_quit.triggered.connect(lambda: (listener.stop(), app.quit()))
    tray_menu.addAction(act_quit)

    tray.setContextMenu(tray_menu)

    def on_tray_activated(reason):
        if reason in (QSystemTrayIcon.ActivationReason.Trigger, QSystemTrayIcon.ActivationReason.DoubleClick):
            if widget.isVisible():
                widget.hide()
            else:
                widget.show()
                widget.raise_()
                widget.activateWindow()

    tray.activated.connect(on_tray_activated)
    tray.show()

    sys.exit(app.exec())
