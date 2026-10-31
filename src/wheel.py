"""
Radial Wheel Widget Component for Mossambi.
Renders the circular petal layout, center hub, animations, and icons.
Directly implements the design from Image reference.png and Design.md.
"""

import math
import os
from typing import List, Optional, Tuple
from PyQt6.QtCore import Qt, QPointF, QRectF, QTimer, pyqtSignal
from PyQt6.QtGui import (
    QPainter, QPainterPath, QColor, QPen, QBrush, QFont,
    QMouseEvent, QPaintEvent
)
from PyQt6.QtWidgets import QWidget

from .theme import Theme, get_ui_font
from .icons import (
    draw_format_icon, draw_arrow_up_icon, draw_checkmark_icon,
    draw_error_icon, draw_doc_icon
)
from .registry import DEFAULT_IDLE_SECTORS


class WheelState:
    IDLE = "idle"
    FILE_HOVER = "file_hover"
    HOVER_SECTOR = "hover_sector"
    CONVERTING = "converting"
    SUCCESS = "success"
    ERROR = "error"


class RadialWheel(QWidget):
    """
    Precision vector radial wheel with rounded petals, center hub,
    and state transitions.
    """

    sector_selected = pyqtSignal(str)   # Format selected (e.g. "JPG")
    center_clicked = pyqtSignal()       # Center hub clicked (e.g. choose file)
    widget_dragged = pyqtSignal(QPointF) # Drag-to-move offset

    def __init__(self, theme: Theme, parent: Optional[QWidget] = None):
        super().__init__(parent)
        self.theme = theme
        self.sectors: List[str] = list(DEFAULT_IDLE_SECTORS)
        self.state: str = WheelState.IDLE

        # File and conversion tracking
        self.current_files: List[str] = []
        self.selected_format: Optional[str] = None
        self.hovered_index: Optional[int] = None
        self.is_center_hovered: bool = False
        self.conversion_progress: int = 0
        self.result_message: str = ""
        self.last_output_path: str = ""

        # Window drag tracking
        self._is_dragging_widget: bool = False
        self._drag_start_pos: Optional[QPointF] = None

        # Reset timer after success/error
        self.reset_timer = QTimer(self)
        self.reset_timer.setSingleShot(True)
        self.reset_timer.timeout.connect(self.reset_to_idle)

        # Smooth animation timer
        self.anim_timer = QTimer(self)
        self.anim_timer.setInterval(16)  # ~60 FPS
        self.anim_timer.timeout.connect(self._on_anim_tick)
        self.anim_progress: float = 0.0

        # Enable mouse tracking for hover effects
        self.setMouseTracking(True)

    def set_theme(self, theme: Theme):
        self.theme = theme
        self.update()

    def set_sectors(self, sectors: List[str]):
        if self.sectors != sectors:
            self.sectors = sectors
            self.hovered_index = None
            self.update()

    def set_current_files(self, files: List[str]):
        self.current_files = files
        if files:
            if self.state in (WheelState.IDLE, WheelState.FILE_HOVER):
                self.state = WheelState.FILE_HOVER
        else:
            if self.state == WheelState.FILE_HOVER:
                self.state = WheelState.IDLE
        self.update()

    def start_conversion(self, target_format: str):
        self.selected_format = target_format
        self.state = WheelState.CONVERTING
        self.conversion_progress = 0
        self.anim_progress = 0.0
        self.anim_timer.start()
        self.update()

    def update_progress(self, progress: int):
        self.conversion_progress = max(0, min(100, progress))
        self.update()

    def set_success(self, output_path: str):
        self.state = WheelState.SUCCESS
        self.anim_timer.stop()
        self.last_output_path = output_path
        self.result_message = "Converted"
        self.update()
        self.reset_timer.start(3500)  # Auto reset in 3.5s

    def set_error(self, message: str):
        self.state = WheelState.ERROR
        self.anim_timer.stop()
        self.result_message = message or "Conversion failed"
        self.update()
        self.reset_timer.start(5000)

    def reset_to_idle(self):
        self.state = WheelState.IDLE
        self.selected_format = None
        self.hovered_index = None
        self.conversion_progress = 0
        self.update()

    def _on_anim_tick(self):
        # Smooth interpolation of progress
        if self.state == WheelState.CONVERTING:
            target = float(self.conversion_progress)
            diff = target - self.anim_progress
            if abs(diff) > 0.5:
                self.anim_progress += diff * 0.15
                self.update()

    # Geometry calculations for botanical Mossambi citrus fruit slice
    def _get_radii(self) -> Tuple[float, float, float, float, float, float, float]:
        """Returns (cx, cy, r_hub, r_in, r_out, r_pith, r_peel)."""
        w = self.width()
        h = self.height()
        cx = w / 2.0
        cy = h / 2.0
        r_hub = w * 0.135         # Central Columella core hub (~59px at 440px)
        r_in = w * 0.160          # Carpel pulp inner boundary (~70px)
        r_out = w * 0.415         # Carpel pulp outer boundary (~182px)
        r_pith = w * 0.428        # Albedo inner white pith ring (~188px)
        r_peel = w * 0.444        # Flavedo outer peel rind boundary (~195px)
        return cx, cy, r_hub, r_in, r_out, r_pith, r_peel

    def _build_petal_path(
        self,
        cx: float,
        cy: float,
        r_in: float,
        r_out: float,
        start_deg: float,
        end_deg: float,
        corner_r: float = 12.0
    ) -> QPainterPath:
        """Constructs precision rounded petal wedge with smooth corners."""
        path = QPainterPath()
        span = end_deg - start_deg

        theta_out = math.degrees(corner_r / r_out)
        theta_in = math.degrees((corner_r * 0.7) / r_in)

        # Helper point calculator
        def pt(r: float, deg: float) -> QPointF:
            rad = math.radians(deg)
            return QPointF(cx + r * math.cos(rad), cy - r * math.sin(rad))

        # Corner 1: Outer Start
        p_out_start_1 = pt(r_out - corner_r, start_deg)
        p_out_start_corner = pt(r_out, start_deg)
        p_out_start_2 = pt(r_out, start_deg + theta_out)

        # Corner 2: Outer End
        p_out_end_1 = pt(r_out, end_deg - theta_out)
        p_out_end_corner = pt(r_out, end_deg)
        p_out_end_2 = pt(r_out - corner_r, end_deg)

        # Corner 3: Inner End
        p_in_end_1 = pt(r_in + corner_r * 0.7, end_deg)
        p_in_end_corner = pt(r_in, end_deg)
        p_in_end_2 = pt(r_in, end_deg - theta_in)

        # Corner 4: Inner Start
        p_in_start_1 = pt(r_in, start_deg + theta_in)
        p_in_start_corner = pt(r_in, start_deg)
        p_in_start_2 = pt(r_in + corner_r * 0.7, start_deg)

        path.moveTo(p_out_start_2)
        rect_out = QRectF(cx - r_out, cy - r_out, r_out * 2, r_out * 2)
        path.arcTo(rect_out, start_deg + theta_out, span - 2 * theta_out)
        path.quadTo(p_out_end_corner, p_out_end_2)
        path.lineTo(p_in_end_1)
        path.quadTo(p_in_end_corner, p_in_end_2)
        rect_in = QRectF(cx - r_in, cy - r_in, r_in * 2, r_in * 2)
        path.arcTo(rect_in, end_deg - theta_in, -(span - 2 * theta_in))
        path.quadTo(p_in_start_corner, p_in_start_2)
        path.lineTo(p_out_start_1)
        path.quadTo(p_out_start_corner, p_out_start_2)
        path.closeSubpath()
        return path

    def paintEvent(self, event: QPaintEvent):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setRenderHint(QPainter.RenderHint.TextAntialiasing)
        painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)

        cx, cy, r_hub, r_in, r_out, r_pith, r_peel = self._get_radii()
        n = len(self.sectors)
        if n == 0:
            painter.end()
            return

        step = 360.0 / n
        gap_deg = 3.6  # Consistent angular gap between petals

        # 1. Subtle ambient drop shadow beneath the citrus wheel
        shadow_path = QPainterPath()
        shadow_path.addEllipse(QPointF(cx, cy + 3), r_peel + 5, r_peel + 5)
        painter.fillPath(shadow_path, self.theme.shadow)

        # 2. Botanical Mossambi Outer Flavedo (Peel / Rind) & Albedo (Inner White Pith)
        # Draw Flavedo Outer Peel Rind in Mossambi Green (Pantone 376 C)
        peel_pen = QPen(QColor(255, 255, 255, 80) if not self.theme.is_dark else QColor(255, 255, 255, 30), 1.0)
        painter.setBrush(QBrush(self.theme.accent))
        painter.setPen(peel_pen)
        painter.drawEllipse(QPointF(cx, cy), r_peel, r_peel)

        # Draw Albedo Inner White Pith Layer
        pith_color = QColor(255, 255, 255, 220) if not self.theme.is_dark else QColor(46, 46, 52, 230)
        painter.setBrush(QBrush(pith_color))
        painter.setPen(QPen(self.theme.divider, 0.8))
        painter.drawEllipse(QPointF(cx, cy), r_pith, r_pith)

        # 3. Draw each radial carpel pulp sector (the 8 petals)
        for i, fmt in enumerate(self.sectors):
            # 12 o'clock center offset: center_angle = -90 + i * step
            # Qt angle system: 0 is 3 o'clock, positive is counter-clockwise.
            # To go clockwise from 12 o'clock: angle = 90 - i * step
            mid_angle = 90.0 - i * step
            start_angle = mid_angle - (step - gap_deg) / 2.0
            end_angle = mid_angle + (step - gap_deg) / 2.0

            petal_path = self._build_petal_path(
                cx, cy, r_in, r_out, start_angle, end_angle, corner_r=13.0
            )

            is_hovered = (self.hovered_index == i)
            is_active_converting = (self.state == WheelState.CONVERTING and self.selected_format == fmt)
            is_all_sector = (fmt == "ALL")

            # Determine colors matching Design.md and Image reference.png
            if is_all_sector:
                # Golden Pantone 1235 C Sector
                if is_hovered or is_active_converting:
                    fill_color = self.theme.gold_all_sector.lighter(130) if not self.theme.is_dark else QColor(70, 52, 14, 230)
                    border_color = self.theme.gold_all_sector
                    border_width = 2.0
                    text_color = self.theme.gold_all_sector.darker(110) if not self.theme.is_dark else self.theme.gold_all_sector
                    icon_color = text_color
                else:
                    fill_color = QColor(255, 249, 230, 240) if not self.theme.is_dark else QColor(42, 36, 26, 245)
                    border_color = self.theme.gold_all_sector
                    border_width = 1.6
                    text_color = self.theme.gold_all_sector.darker(120) if not self.theme.is_dark else self.theme.gold_all_sector
                    icon_color = text_color
            elif is_hovered or is_active_converting:
                fill_color = self.theme.segment_hover
                border_color = self.theme.accent
                border_width = 1.5
                text_color = self.theme.accent_hover if self.theme.is_dark else QColor("#3B6419")
                icon_color = text_color
            else:
                fill_color = self.theme.segment_surface
                border_color = self.theme.divider
                border_width = 1.0
                text_color = self.theme.primary_text
                icon_color = self.theme.primary_text

            # Draw petal surface
            painter.setBrush(QBrush(fill_color))
            painter.setPen(QPen(border_color, border_width))
            painter.drawPath(petal_path)

            # Draw icon and label inside petal
            # Center point of petal
            mid_rad = math.radians(mid_angle)
            mid_r = (r_in + r_out) / 2.0
            petal_cx = cx + mid_r * math.cos(mid_rad)
            petal_cy = cy - mid_r * math.sin(mid_rad)

            # Draw Format Icon (offset slightly inward)
            icon_y = petal_cy - 10
            draw_format_icon(painter, fmt, petal_cx, icon_y, size=21, color=icon_color)

            # Draw Format Label (offset slightly outward)
            painter.setPen(text_color)
            painter.setFont(get_ui_font(size=12, bold=True))
            label_rect = QRectF(petal_cx - 40, petal_cy + 5, 80, 20)
            painter.drawText(label_rect, Qt.AlignmentFlag.AlignCenter, fmt)

        # 3. Draw Center Hub
        hub_path = QPainterPath()
        hub_path.addEllipse(QPointF(cx, cy), r_hub, r_hub)

        # Hub surface
        hub_bg = self.theme.wheel_surface
        hub_border = self.theme.accent if (self.is_center_hovered or self.state == WheelState.CONVERTING) else self.theme.divider
        painter.setBrush(QBrush(hub_bg))
        painter.setPen(QPen(hub_border, 1.2))
        painter.drawPath(hub_path)

        # Hub Content by State
        if self.state == WheelState.CONVERTING:
            # Draw circular progress ring
            progress_rect = QRectF(cx - r_hub + 6, cy - r_hub + 6, (r_hub - 6) * 2, (r_hub - 6) * 2)
            # Track ring (subtle gray)
            painter.setPen(QPen(self.theme.divider, 5.0, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
            painter.setBrush(Qt.BrushStyle.NoBrush)
            painter.drawEllipse(progress_rect)

            # Active green progress arc
            progress_val = max(self.anim_progress, float(self.conversion_progress))
            span_angle = int(-progress_val * 3.6 * 16)  # Qt uses 1/16th degrees, negative = clockwise
            painter.setPen(QPen(self.theme.accent, 5.2, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap))
            painter.drawArc(progress_rect, 90 * 16, span_angle)

            # Percentage text
            painter.setPen(self.theme.primary_text)
            painter.setFont(get_ui_font(size=17, bold=True, tabular=True))
            p_rect = QRectF(cx - r_hub, cy - 14, r_hub * 2, 22)
            painter.drawText(p_rect, Qt.AlignmentFlag.AlignCenter, f"{int(progress_val)}%")

            # Substatus text
            painter.setPen(self.theme.secondary_text)
            painter.setFont(get_ui_font(size=10, bold=False))
            sub_rect = QRectF(cx - r_hub, cy + 9, r_hub * 2, 16)
            painter.drawText(sub_rect, Qt.AlignmentFlag.AlignCenter, "Converting...")

        elif self.state == WheelState.SUCCESS:
            # Green checkmark badge
            draw_checkmark_icon(painter, cx, cy - 12, size=30, circle_color=self.theme.accent)

            painter.setPen(self.theme.primary_text)
            painter.setFont(get_ui_font(size=12, bold=True))
            txt_rect = QRectF(cx - r_hub, cy + 8, r_hub * 2, 18)
            painter.drawText(txt_rect, Qt.AlignmentFlag.AlignCenter, self.result_message or "Converted")

            # File count or name
            painter.setPen(self.theme.secondary_text)
            painter.setFont(get_ui_font(size=10, bold=False))
            sub_rect = QRectF(cx - r_hub, cy + 24, r_hub * 2, 16)
            count = len(self.current_files)
            file_sub = f"{count} files" if count > 1 else (os.path.basename(self.last_output_path) if self.last_output_path else "Ready")
            if len(file_sub) > 14:
                file_sub = file_sub[:12] + "..."
            painter.drawText(sub_rect, Qt.AlignmentFlag.AlignCenter, file_sub)

        elif self.state == WheelState.ERROR:
            draw_error_icon(painter, cx, cy - 12, size=30, circle_color=self.theme.error)

            painter.setPen(self.theme.error)
            painter.setFont(get_ui_font(size=12, bold=True))
            txt_rect = QRectF(cx - r_hub, cy + 8, r_hub * 2, 18)
            painter.drawText(txt_rect, Qt.AlignmentFlag.AlignCenter, "Failed")

            painter.setPen(self.theme.secondary_text)
            painter.setFont(get_ui_font(size=9, bold=False))
            sub_rect = QRectF(cx - r_hub, cy + 24, r_hub * 2, 16)
            painter.drawText(sub_rect, Qt.AlignmentFlag.AlignCenter, "Click to retry")

        elif self.hovered_index is not None and 0 <= self.hovered_index < len(self.sectors):
            # Target format selected
            target_fmt = self.sectors[self.hovered_index]
            draw_format_icon(painter, target_fmt, cx, cy - 14, size=24, color=self.theme.accent)

            painter.setPen(self.theme.accent)
            painter.setFont(get_ui_font(size=13, bold=True))
            fmt_rect = QRectF(cx - r_hub, cy + 1, r_hub * 2, 18)
            painter.drawText(fmt_rect, Qt.AlignmentFlag.AlignCenter, f"To {target_fmt}")

            painter.setPen(self.theme.secondary_text)
            painter.setFont(get_ui_font(size=10, bold=False))
            hint_rect = QRectF(cx - r_hub, cy + 18, r_hub * 2, 16)
            painter.drawText(hint_rect, Qt.AlignmentFlag.AlignCenter, "Drop to convert")

        elif self.state == WheelState.FILE_HOVER and self.current_files:
            # File loaded, waiting for sector selection
            first_file = self.current_files[0]
            filename = os.path.basename(first_file)
            if len(filename) > 13:
                filename = filename[:11] + ".."

            draw_doc_icon(painter, cx, cy - 14, size=22, color=self.theme.primary_text)

            painter.setPen(self.theme.primary_text)
            painter.setFont(get_ui_font(size=11, bold=True))
            name_rect = QRectF(cx - r_hub, cy + 1, r_hub * 2, 18)
            painter.drawText(name_rect, Qt.AlignmentFlag.AlignCenter, filename)

            painter.setPen(self.theme.secondary_text)
            painter.setFont(get_ui_font(size=10, bold=False))
            hint_rect = QRectF(cx - r_hub, cy + 18, r_hub * 2, 16)
            painter.drawText(hint_rect, Qt.AlignmentFlag.AlignCenter, "Pick format")

        else:
            # Idle State (Image 1 in Image reference.png)
            draw_arrow_up_icon(painter, cx, cy - 12, size=22, color=self.theme.secondary_text)

            painter.setPen(self.theme.primary_text)
            painter.setFont(get_ui_font(size=11, bold=False))
            text_rect = QRectF(cx - r_hub, cy + 8, r_hub * 2, 32)
            painter.drawText(text_rect, Qt.AlignmentFlag.AlignCenter, "Drop files\nhere")

        painter.end()

    # Polar Hit Testing
    def _hit_test(self, pos: QPointF) -> Tuple[Optional[int], bool]:
        """Maps cursor (x, y) to sector index or center hub flag."""
        cx, cy, r_hub, r_in, r_out, r_pith, r_peel = self._get_radii()
        dx = pos.x() - cx
        dy = cy - pos.y()  # standard math Cartesian y (upward is positive)
        r = math.hypot(dx, dy)

        # Center hub hit test
        if r <= r_hub:
            return None, True

        # Petal sector hit test (including outer carpel margin)
        if r_in <= r <= r_peel + 6:
            n = len(self.sectors)
            if n == 0:
                return None, False

            # Convert to angle in degrees from 0 to 360, where 90 is 12 o'clock
            angle = math.degrees(math.atan2(dy, dx))
            if angle < 0:
                angle += 360.0

            # Clockwise angle from 12 o'clock (90 deg)
            cw_from_12 = (90.0 - angle) % 360.0
            step = 360.0 / n

            # Sector index
            idx = int((cw_from_12 + step / 2.0) / step) % n
            return idx, False

        return None, False

    def mouseMoveEvent(self, event: QMouseEvent):
        if self._is_dragging_widget and self._drag_start_pos:
            delta = event.globalPosition() - self._drag_start_pos
            self.widget_dragged.emit(delta)
            return

        sector_idx, is_hub = self._hit_test(event.position())
        changed = False

        if self.hovered_index != sector_idx:
            self.hovered_index = sector_idx
            changed = True

        if self.is_center_hovered != is_hub:
            self.is_center_hovered = is_hub
            changed = True

        if changed:
            self.update()

    def mousePressEvent(self, event: QMouseEvent):
        if event.button() == Qt.MouseButton.LeftButton:
            sector_idx, is_hub = self._hit_test(event.position())

            if is_hub:
                # Center clicked
                self.center_clicked.emit()
            elif sector_idx is not None and 0 <= sector_idx < len(self.sectors):
                # Sector clicked
                selected_fmt = self.sectors[sector_idx]
                self.sector_selected.emit(selected_fmt)
            else:
                # Clicked on background or border: initiate window dragging
                self._is_dragging_widget = True
                self._drag_start_pos = event.globalPosition()

    def mouseReleaseEvent(self, event: QMouseEvent):
        if event.button() == Qt.MouseButton.LeftButton:
            self._is_dragging_widget = False
            self._drag_start_pos = None

    def leaveEvent(self, event):
        self.hovered_index = None
        self.is_center_hovered = False
        self.update()
