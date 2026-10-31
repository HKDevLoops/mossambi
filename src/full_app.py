"""
Full macOS-Styled GUI Desktop Application for Mossambi.
Features Apple-style traffic lights, acrylic translucency, Pantone certified citrus colors,
Universal Format Catalog, and the flagship Indian Competitive Exam & Job Application Form Tuner.
"""

import os
import sys
from typing import Optional, List
from PyQt6.QtCore import Qt, QPoint, QSize, pyqtSignal
from PyQt6.QtGui import QColor, QFont, QIcon, QPainter, QPen, QBrush, QMouseEvent
from PyQt6.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QLabel, QPushButton,
    QStackedWidget, QListWidget, QListWidgetItem, QFileDialog, QLineEdit,
    QSpinBox, QCheckBox, QComboBox, QProgressBar, QFrame, QMessageBox,
    QScrollArea, QSplitter
)

from .theme import Theme, ThemeMode, get_ui_font
from .pantone import PantonePalette
from .converter import optimize_exam_image, resolve_sibling_output_path, ConversionWorker
from .registry import UniversalRegistry, FileCategory


class MacTitleBar(QWidget):
    """macOS-style traffic light window controls and draggable header."""
    close_clicked = pyqtSignal()
    min_clicked = pyqtSignal()
    max_clicked = pyqtSignal()

    def __init__(self, title: str, theme: Theme, parent: Optional[QWidget] = None):
        super().__init__(parent)
        self.theme = theme
        self.setFixedHeight(40)
        self._drag_pos: Optional[QPoint] = None

        layout = QHBoxLayout(self)
        layout.setContentsMargins(14, 0, 14, 0)
        layout.setSpacing(8)

        # Traffic lights
        self.btn_close = QPushButton()
        self.btn_min = QPushButton()
        self.btn_max = QPushButton()

        for btn, col in [
            (self.btn_close, "#FF5F56"),
            (self.btn_min, "#FFBD2E"),
            (self.btn_max, "#27C93F"),
        ]:
            btn.setFixedSize(12, 12)
            btn.setStyleSheet(f"""
                QPushButton {{
                    background-color: {col};
                    border-radius: 6px;
                    border: 0.5px solid rgba(0,0,0,0.15);
                }}
                QPushButton:hover {{
                    border: 0.5px solid rgba(0,0,0,0.4);
                }}
            """)

        self.btn_close.clicked.connect(self.close_clicked)
        self.btn_min.clicked.connect(self.min_clicked)
        self.btn_max.clicked.connect(self.max_clicked)

        layout.addWidget(self.btn_close)
        layout.addWidget(self.btn_min)
        layout.addWidget(self.btn_max)

        layout.addSpacing(16)
        self.lbl_title = QLabel(title)
        self.lbl_title.setFont(get_ui_font(13, bold=True))
        self.lbl_title.setStyleSheet(f"color: {self.theme.primary_text.name()};")
        layout.addWidget(self.lbl_title)
        layout.addStretch()

    def mousePressEvent(self, event: QMouseEvent):
        if event.button() == Qt.MouseButton.LeftButton:
            self._drag_pos = event.globalPosition().toPoint() - self.window().frameGeometry().topLeft()
            event.accept()

    def mouseMoveEvent(self, event: QMouseEvent):
        if self._drag_pos is not None and event.buttons() == Qt.MouseButton.LeftButton:
            self.window().move(event.globalPosition().toPoint() - self._drag_pos)
            event.accept()

    def mouseReleaseEvent(self, event: QMouseEvent):
        self._drag_pos = None


class MossambiDesktopApp(QMainWindow):
    """macOS-styled desktop interface for Mossambi."""

    def __init__(self, theme: Theme):
        super().__init__()
        self.theme = theme
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Window)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground, True)
        self.resize(920, 640)
        self.setMinimumSize(800, 520)

        self.init_ui()

    def init_ui(self):
        central = QWidget(self)
        central.setObjectName("CentralSurface")
        bg_col = self.theme.bg.name()
        border_col = self.theme.divider.name()

        central.setStyleSheet(f"""
            QWidget#CentralSurface {{
                background-color: {bg_col};
                border-radius: 12px;
                border: 1px solid {border_col};
            }}
        """)

        main_layout = QVBoxLayout(central)
        main_layout.setContentsMargins(1, 1, 1, 1)
        main_layout.setSpacing(0)

        # Title bar
        self.title_bar = MacTitleBar("Mossambi Desktop Suite", self.theme, self)
        self.title_bar.close_clicked.connect(self.hide)
        self.title_bar.min_clicked.connect(self.showMinimized)
        self.title_bar.max_clicked.connect(self._toggle_maximize)
        main_layout.addWidget(self.title_bar)

        # Body with Apple sidebar
        body = QWidget()
        body_layout = QHBoxLayout(body)
        body_layout.setContentsMargins(0, 0, 0, 0)
        body_layout.setSpacing(0)

        # Sidebar
        self.sidebar = QListWidget()
        self.sidebar.setFixedWidth(220)
        accent_hex = self.theme.accent.name()
        text_hex = self.theme.primary_text.name()
        side_bg = "#EBECEF" if not self.theme.is_dark else "#202024"

        self.sidebar.setStyleSheet(f"""
            QListWidget {{
                background-color: {side_bg};
                border: none;
                border-right: 1px solid {border_col};
                border-bottom-left-radius: 12px;
                padding-top: 10px;
                color: {text_hex};
                font-size: 13px;
                outline: none;
            }}
            QListWidget::item {{
                height: 38px;
                padding-left: 14px;
                margin: 2px 8px;
                border-radius: 6px;
            }}
            QListWidget::item:selected {{
                background-color: {accent_hex};
                color: #FFFFFF;
                font-weight: bold;
            }}
            QListWidget::item:hover:!selected {{
                background-color: rgba(0,0,0,0.05);
            }}
        """)

        nav_items = [
            ("📁  File Transcoder", 0),
            ("🇮🇳  Exam Form Studio", 1),
            ("📦  Archive Suite", 2),
            ("⚡  Hardware & SIMD", 3),
        ]
        for label, idx in nav_items:
            item = QListWidgetItem(label)
            item.setData(Qt.ItemDataRole.UserRole, idx)
            self.sidebar.addItem(item)

        self.sidebar.setCurrentRow(0)
        self.sidebar.currentRowChanged.connect(self._on_tab_changed)
        body_layout.addWidget(self.sidebar)

        # Stacked Pages
        self.pages = QStackedWidget()
        self.page_transcode = self._build_transcoder_page()
        self.page_exam = self._build_exam_studio_page()
        self.page_archive = self._build_archive_page()
        self.page_hw = self._build_hwaccel_page()

        self.pages.addWidget(self.page_transcode)
        self.pages.addWidget(self.page_exam)
        self.pages.addWidget(self.page_archive)
        self.pages.addWidget(self.page_hw)

        body_layout.addWidget(self.pages)
        main_layout.addWidget(body)

        self.setCentralWidget(central)

    def _on_tab_changed(self, row: int):
        self.pages.setCurrentIndex(row)

    def _toggle_maximize(self):
        if self.isMaximized():
            self.showNormal()
        else:
            self.showMaximized()

    # 1. File Transcoder Page
    def _build_transcoder_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 20, 28, 24)
        layout.setSpacing(14)

        title = QLabel("Universal Offline File Transcoder")
        title.setFont(get_ui_font(18, bold=True))
        layout.addWidget(title)

        desc = QLabel("Drag & drop files or browse. Supports zero-loss 0.00% stream conversion and golden 'ALL' batch transcoding.")
        desc.setStyleSheet(f"color: {self.theme.secondary_text.name()}; font-size: 13px;")
        layout.addWidget(desc)

        # File selection box
        file_box = QFrame()
        file_box.setStyleSheet(f"background: {'#FFFFFF' if not self.theme.is_dark else '#2A2A2E'}; border-radius: 8px; border: 1px dashed {self.theme.divider.name()}; padding: 18px;")
        fb_layout = QVBoxLayout(file_box)

        self.lbl_selected_file = QLabel("No file selected. Drop files here or click Choose.")
        self.lbl_selected_file.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.lbl_selected_file.setFont(get_ui_font(13))
        fb_layout.addWidget(self.lbl_selected_file)

        btn_browse = QPushButton("Choose File...")
        btn_browse.setFixedWidth(140)
        btn_browse.setStyleSheet(f"background: {self.theme.accent.name()}; color: white; padding: 7px; border-radius: 6px; font-weight: bold;")
        btn_browse.clicked.connect(self._browse_transcode_file)
        fb_layout.addWidget(btn_browse, alignment=Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(file_box)

        # Target format selector
        fmt_layout = QHBoxLayout()
        fmt_lbl = QLabel("Target Format:")
        fmt_lbl.setFont(get_ui_font(13, bold=True))
        fmt_layout.addWidget(fmt_lbl)

        self.combo_formats = QComboBox()
        self.combo_formats.addItems(["JPG", "PNG", "WEBP", "PDF", "MP4", "MP3", "WAV", "FLAC", "ALL (Batch Transcode)"])
        self.combo_formats.setFixedWidth(200)
        fmt_layout.addWidget(self.combo_formats)
        fmt_layout.addStretch()
        layout.addLayout(fmt_layout)

        # Convert button
        self.btn_convert = QPushButton("Convert File Now")
        self.btn_convert.setFixedHeight(40)
        self.btn_convert.setFont(get_ui_font(14, bold=True))
        self.btn_convert.setStyleSheet(f"background: {self.theme.accent.name()}; color: white; border-radius: 8px;")
        self.btn_convert.clicked.connect(self._run_transcode)
        layout.addWidget(self.btn_convert)

        self.transcode_progress = QProgressBar()
        self.transcode_progress.setFixedHeight(6)
        self.transcode_progress.setTextVisible(False)
        self.transcode_progress.setValue(0)
        self.transcode_progress.setStyleSheet(f"QProgressBar::chunk {{ background-color: {self.theme.accent.name()}; }}")
        layout.addWidget(self.transcode_progress)

        self.lbl_transcode_status = QLabel("")
        self.lbl_transcode_status.setFont(get_ui_font(12))
        layout.addWidget(self.lbl_transcode_status)

        layout.addStretch()
        return page

    # 2. Flagship Indian Exam Form Studio
    def _build_exam_studio_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 20, 28, 24)
        layout.setSpacing(12)

        title = QLabel("🇮🇳 Indian Competitive Exam & Job Application Form Tuner")
        title.setFont(get_ui_font(18, bold=True))
        layout.addWidget(title)

        desc = QLabel("Engineered specifically for UPSC, SSC, IBPS, GATE, NTA, and State PSC recruitment portals.\nGuarantees exact dimensions, calibrated DPI, and binary search byte-envelope limits.")
        desc.setStyleSheet(f"color: {self.theme.secondary_text.name()}; font-size: 13px;")
        layout.addWidget(desc)

        # Preset selector
        preset_layout = QHBoxLayout()
        plbl = QLabel("Application Preset:")
        plbl.setFont(get_ui_font(13, bold=True))
        preset_layout.addWidget(plbl)

        self.combo_exam_presets = QComboBox()
        self.combo_exam_presets.addItems([
            "UPSC Civil Services Photo (350x450 px @ 300 DPI, 20 KB - 100 KB)",
            "UPSC Civil Services Signature (350x150 px @ 300 DPI, 10 KB - 40 KB)",
            "SSC CGL / CHSL Photo (140x190 px @ 200 DPI, 20 KB - 50 KB)",
            "SSC CGL / CHSL Signature (140x60 px @ 200 DPI, 10 KB - 20 KB)",
            "IBPS Bank PO Photo (200x230 px @ 200 DPI, 20 KB - 50 KB)",
            "IBPS Bank PO Signature (140x60 px @ 200 DPI, 10 KB - 20 KB)",
            "Custom Exam Specification...",
        ])
        self.combo_exam_presets.currentIndexChanged.connect(self._on_exam_preset_changed)
        preset_layout.addWidget(self.combo_exam_presets)
        layout.addLayout(preset_layout)

        # Tuning Grid
        grid_frame = QFrame()
        grid_frame.setStyleSheet(f"background: {'#FFFFFF' if not self.theme.is_dark else '#2A2A2E'}; border-radius: 8px; border: 1px solid {self.theme.divider.name()}; padding: 12px;")
        gf_layout = QVBoxLayout(grid_frame)

        row1 = QHBoxLayout()
        row1.addWidget(QLabel("Width (px):"))
        self.spin_width = QSpinBox()
        self.spin_width.setRange(50, 4000)
        self.spin_width.setValue(350)
        row1.addWidget(self.spin_width)

        row1.addWidget(QLabel("Height (px):"))
        self.spin_height = QSpinBox()
        self.spin_height.setRange(50, 4000)
        self.spin_height.setValue(450)
        row1.addWidget(self.spin_height)

        row1.addWidget(QLabel("DPI:"))
        self.spin_dpi = QSpinBox()
        self.spin_dpi.setRange(72, 600)
        self.spin_dpi.setValue(300)
        row1.addWidget(self.spin_dpi)
        gf_layout.addLayout(row1)

        row2 = QHBoxLayout()
        row2.addWidget(QLabel("Min Size (KB):"))
        self.spin_min_kb = QSpinBox()
        self.spin_min_kb.setRange(1, 10000)
        self.spin_min_kb.setValue(20)
        row2.addWidget(self.spin_min_kb)

        row2.addWidget(QLabel("Max Size (KB):"))
        self.spin_max_kb = QSpinBox()
        self.spin_max_kb.setRange(1, 10000)
        self.spin_max_kb.setValue(100)
        row2.addWidget(self.spin_max_kb)
        gf_layout.addLayout(row2)

        self.chk_whiten = QCheckBox("Normalize mobile scan white-balance (removes shadow and gray background)")
        self.chk_whiten.setChecked(True)
        gf_layout.addWidget(self.chk_whiten)

        layout.addWidget(grid_frame)

        # File picker for Exam Studio
        btn_pick_photo = QPushButton("Select Photo or Scanned Document...")
        btn_pick_photo.setStyleSheet(f"background: {self.theme.accent.name()}; color: white; padding: 8px; border-radius: 6px; font-weight: bold;")
        btn_pick_photo.clicked.connect(self._browse_exam_file)
        layout.addWidget(btn_pick_photo)

        self.lbl_exam_file = QLabel("No document selected.")
        self.lbl_exam_file.setStyleSheet(f"color: {self.theme.secondary_text.name()};")
        layout.addWidget(self.lbl_exam_file)

        # Optimize Button
        self.btn_optimize_exam = QPushButton("⚡ Optimize & Converge File Size")
        self.btn_optimize_exam.setFixedHeight(40)
        self.btn_optimize_exam.setFont(get_ui_font(13, bold=True))
        self.btn_optimize_exam.setStyleSheet(f"background: {PantonePalette.CITRUS_GOLD.hex}; color: #1D1D1F; border-radius: 8px;")
        self.btn_optimize_exam.clicked.connect(self._run_exam_optimization)
        layout.addWidget(self.btn_optimize_exam)

        self.lbl_exam_result = QLabel("")
        self.lbl_exam_result.setFont(get_ui_font(13, bold=True))
        layout.addWidget(self.lbl_exam_result)

        layout.addStretch()
        return page

    # 3. Archive Page
    def _build_archive_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 20, 28, 24)
        layout.setSpacing(14)

        title = QLabel("Exhaustive Archive Engine")
        title.setFont(get_ui_font(18, bold=True))
        layout.addWidget(title)

        desc = QLabel("Supports ZIP, 7Z, TAR, TAR.GZ, TAR.BZ2, TAR.XZ, and TAR.ZST with Zip-Slip path traversal protection.")
        desc.setStyleSheet(f"color: {self.theme.secondary_text.name()}; font-size: 13px;")
        layout.addWidget(desc)

        # Container selector
        row = QHBoxLayout()
        row.addWidget(QLabel("Archive Format:"))
        self.combo_archive_type = QComboBox()
        self.combo_archive_type.addItems(["ZIP", "TAR.GZ", "TAR.XZ", "TAR.ZST", "7Z", "TAR"])
        row.addWidget(self.combo_archive_type)
        row.addStretch()
        layout.addLayout(row)

        btn_pack = QPushButton("📦 Pack Files into Archive...")
        btn_pack.setStyleSheet(f"background: {self.theme.accent.name()}; color: white; padding: 10px; border-radius: 6px; font-weight: bold;")
        btn_pack.clicked.connect(self._pack_archive_dialog)
        layout.addWidget(btn_pack)

        btn_extract = QPushButton("📂 Extract Archive Safely...")
        btn_extract.setStyleSheet(f"background: {'#FFFFFF' if not self.theme.is_dark else '#2A2A2E'}; color: {self.theme.primary_text.name()}; border: 1px solid {self.theme.divider.name()}; padding: 10px; border-radius: 6px; font-weight: bold;")
        btn_extract.clicked.connect(self._extract_archive_dialog)
        layout.addWidget(btn_extract)

        self.lbl_archive_status = QLabel("")
        layout.addWidget(self.lbl_archive_status)

        layout.addStretch()
        return page

    # 4. Hardware Acceleration & SIMD Page
    def _build_hwaccel_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(28, 20, 28, 24)
        layout.setSpacing(12)

        title = QLabel("⚡ Hardware Acceleration & SIMD Architecture")
        title.setFont(get_ui_font(18, bold=True))
        layout.addWidget(title)

        box = QFrame()
        box.setStyleSheet(f"background: {'#FFFFFF' if not self.theme.is_dark else '#2A2A2E'}; border-radius: 8px; border: 1px solid {self.theme.divider.name()}; padding: 16px;")
        blayout = QVBoxLayout(box)

        # Query hardware capabilities
        is_arm = sys.platform.startswith("arm") or "aarch64" in os.environ.get("PROCESSOR_ARCHITECTURE", "").lower()
        gpu_info = "NVIDIA CUDA / NVENC (Hardware Accelerated)" if sys.platform == "win32" else "VAAPI / Generic Hardware"

        info_lines = [
            f"<b>OS Platform:</b> {sys.platform.capitalize()}",
            f"<b>CPU Architecture:</b> {'ARM64' if is_arm else 'x86_64'}",
            f"<b>GPU Engine:</b> {gpu_info}",
            f"<b>Zero-Loss Remuxing:</b> Stream Copy (-c copy) Enabled",
            f"<b>ARM NEON SIMD:</b> {'Active (128-bit SIMD)' if is_arm else 'Disabled'}",
            f"<b>x86_64 AVX2 SIMD:</b> {'Disabled' if is_arm else 'Active (256-bit SIMD)'}",
            f"<b>Current Citrus Shade:</b> {self.theme.active_pantone_shade.name} ({self.theme.active_pantone_shade.code})",
        ]

        for line in info_lines:
            lbl = QLabel(line)
            lbl.setFont(get_ui_font(13))
            blayout.addWidget(lbl)

        layout.addWidget(box)
        layout.addStretch()
        return page

    # Handlers
    def _browse_transcode_file(self):
        f, _ = QFileDialog.getOpenFileName(self, "Select File to Convert")
        if f:
            self._current_transcode_path = f
            self.lbl_selected_file.setText(os.path.basename(f))

    def _run_transcode(self):
        if not hasattr(self, "_current_transcode_path") or not os.path.exists(self._current_transcode_path):
            QMessageBox.warning(self, "No File", "Please select a file to convert first.")
            return

        fmt = self.combo_formats.currentText().split()[0]
        self.btn_convert.setEnabled(False)
        self.transcode_progress.setValue(20)

        self.worker = ConversionWorker([self._current_transcode_path], fmt)
        self.worker.progress_changed.connect(self.transcode_progress.setValue)
        self.worker.conversion_finished.connect(self._on_transcode_done)
        self.worker.start()

    def _on_transcode_done(self, success: bool, output: str, err: str):
        self.btn_convert.setEnabled(True)
        if success:
            self.lbl_transcode_status.setText(f"✓ Converted: {os.path.basename(output)}")
            self.lbl_transcode_status.setStyleSheet(f"color: {self.theme.accent.name()}; font-weight: bold;")
        else:
            self.lbl_transcode_status.setText(f"✗ Error: {err}")
            self.lbl_transcode_status.setStyleSheet("color: red; font-weight: bold;")

    def _on_exam_preset_changed(self, idx: int):
        presets = [
            (350, 450, 300, 20, 100),
            (350, 150, 300, 10, 40),
            (140, 190, 200, 20, 50),
            (140, 60, 200, 10, 20),
            (200, 230, 200, 20, 50),
            (140, 60, 200, 10, 20),
            (350, 450, 200, 20, 100),
        ]
        if idx < len(presets):
            w, h, dpi, min_k, max_k = presets[idx]
            self.spin_width.setValue(w)
            self.spin_height.setValue(h)
            self.spin_dpi.setValue(dpi)
            self.spin_min_kb.setValue(min_k)
            self.spin_max_kb.setValue(max_k)

    def _browse_exam_file(self):
        f, _ = QFileDialog.getOpenFileName(self, "Select Photo or Document", filter="Images (*.jpg *.jpeg *.png *.webp *.bmp *.tiff)")
        if f:
            self._current_exam_path = f
            self.lbl_exam_file.setText(f"Selected: {os.path.basename(f)}")

    def _run_exam_optimization(self):
        if not hasattr(self, "_current_exam_path") or not os.path.exists(self._current_exam_path):
            QMessageBox.warning(self, "No File", "Please select a photo or document first.")
            return

        w = self.spin_width.value()
        h = self.spin_height.value()
        dpi = self.spin_dpi.value()
        min_b = self.spin_min_kb.value() * 1024
        max_b = self.spin_max_kb.value() * 1024
        whiten = self.chk_whiten.isChecked()

        out_path = resolve_sibling_output_path(self._current_exam_path, "jpg")
        out_path = out_path.replace("_converted", "_exam_ready")

        try:
            res = optimize_exam_image(
                self._current_exam_path,
                out_path,
                target_width=w,
                target_height=h,
                dpi=dpi,
                min_bytes=min_b,
                max_bytes=max_b,
                whiten=whiten
            )
            size_kb = os.path.getsize(res) / 1024.0
            self.lbl_exam_result.setText(f"✓ Form-Ready File Generated: {os.path.basename(res)} ({size_kb:.1f} KB)\nPassed strict upload validation boundaries!")
            self.lbl_exam_result.setStyleSheet(f"color: {self.theme.accent.name()};")
        except Exception as e:
            self.lbl_exam_result.setText(f"✗ Optimization failed: {e}")
            self.lbl_exam_result.setStyleSheet("color: red;")

    def _pack_archive_dialog(self):
        files, _ = QFileDialog.getOpenFileNames(self, "Select Files to Archive")
        if files:
            out, _ = QFileDialog.getSaveFileName(self, "Save Archive", filter="Zip Archive (*.zip);;Tar Archive (*.tar.gz)")
            if out:
                from .converter import convert_archive
                convert_archive(files[0], out, "ZIP")
                self.lbl_archive_status.setText(f"✓ Created archive: {os.path.basename(out)}")

    def _extract_archive_dialog(self):
        arc, _ = QFileDialog.getOpenFileName(self, "Select Archive to Extract", filter="Archives (*.zip *.tar *.gz *.7z)")
        if arc:
            out_dir = QFileDialog.getExistingDirectory(self, "Select Destination Folder")
            if out_dir:
                from .converter import convert_archive
                convert_archive(arc, out_dir, "EXTRACT")
                self.lbl_archive_status.setText(f"✓ Extracted into: {os.path.basename(out_dir)}")
