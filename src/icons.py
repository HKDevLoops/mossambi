"""
Vector Icon Drawing Routines for Mossambi Widget.
Clean, Apple-inspired monochrome stroke icons painted via QPainter.
"""

import math
from PyQt6.QtCore import Qt, QPointF, QRectF
from PyQt6.QtGui import QPainter, QPainterPath, QColor, QPen, QBrush


def draw_star_all_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(246, 179, 36)):
    """Sparkle / Star multi-format icon for the golden ALL sector."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.4, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(QBrush(color))
    path = QPainterPath()
    r_outer = size * 0.5
    r_inner = size * 0.18
    for i in range(8):
        angle = i * (math.pi / 4.0)
        r = r_outer if (i % 2 == 0) else r_inner
        x = r * math.cos(angle)
        y = r * math.sin(angle)
        if i == 0:
            path.moveTo(x, y)
        else:
            path.lineTo(x, y)
    path.closeSubpath()
    p.drawPath(path)
    p.restore()


def draw_image_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Clean image frame with mountain and sun."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w, h = size, size * 0.78
    # Outer rounded frame
    p.drawRoundedRect(QRectF(-w / 2, -h / 2, w, h), 3, 3)

    # Sun
    sun_r = 1.8
    p.drawEllipse(QPointF(w / 4, -h / 5), sun_r, sun_r)

    # Mountains
    path = QPainterPath()
    path.moveTo(-w / 2 + 2, h / 2 - 1.5)
    path.lineTo(-w / 6, -h / 6)
    path.lineTo(w / 7, h / 4)
    path.lineTo(w / 2 - 2, h / 2 - 1.5)
    p.drawPath(path)
    p.restore()


def draw_doc_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Document sheet with folded corner and lines."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w = size * 0.72
    h = size
    fold = 4.5

    # Page outline with folded corner
    path = QPainterPath()
    path.moveTo(-w / 2, -h / 2)
    path.lineTo(w / 2 - fold, -h / 2)
    path.lineTo(w / 2, -h / 2 + fold)
    path.lineTo(w / 2, h / 2)
    path.lineTo(-w / 2, h / 2)
    path.closeSubpath()
    p.drawPath(path)

    # Corner fold flap
    fold_path = QPainterPath()
    fold_path.moveTo(w / 2 - fold, -h / 2)
    fold_path.lineTo(w / 2 - fold, -h / 2 + fold)
    fold_path.lineTo(w / 2, -h / 2 + fold)
    p.drawPath(fold_path)

    # Horizontal text lines
    line_pen = QPen(color, 1.3, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap)
    p.setPen(line_pen)
    p.drawLine(QPointF(-w / 2 + 3, -h / 2 + fold + 3), QPointF(w / 2 - 3, -h / 2 + fold + 3))
    p.drawLine(QPointF(-w / 2 + 3, 0), QPointF(w / 2 - 3, 0))
    p.drawLine(QPointF(-w / 2 + 3, h / 4), QPointF(w / 4, h / 4))
    p.restore()


def draw_camera_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Camera icon (HEIC)."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w = size
    h = size * 0.72

    # Camera body with top notch
    path = QPainterPath()
    path.moveTo(-w / 2, -h / 2 + 2)
    path.lineTo(-w / 4, -h / 2 + 2)
    path.lineTo(-w / 5, -h / 2)
    path.lineTo(w / 5, -h / 2)
    path.lineTo(w / 4, -h / 2 + 2)
    path.lineTo(w / 2, -h / 2 + 2)
    path.lineTo(w / 2, h / 2)
    path.lineTo(-w / 2, h / 2)
    path.closeSubpath()
    p.drawPath(path)

    # Lens circle
    p.drawEllipse(QPointF(0, 1), w * 0.22, w * 0.22)
    p.restore()


def draw_layers_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Stacked isometric layers icon (TIFF)."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w = size
    h_layer = size * 0.36

    # Top rhombus layer
    p1 = QPainterPath()
    p1.moveTo(0, -size / 2)
    p1.lineTo(w / 2, -size / 2 + h_layer)
    p1.lineTo(0, -size / 2 + h_layer * 2)
    p1.lineTo(-w / 2, -size / 2 + h_layer)
    p1.closeSubpath()
    p.drawPath(p1)

    # Lower chevron layer
    p2 = QPainterPath()
    p2.moveTo(-w / 2, -size / 2 + h_layer + 4)
    p2.lineTo(0, -size / 2 + h_layer * 2 + 4)
    p2.lineTo(w / 2, -size / 2 + h_layer + 4)
    p.drawPath(p2)

    # Lowest chevron layer
    p3 = QPainterPath()
    p3.moveTo(-w / 2, -size / 2 + h_layer + 8)
    p3.lineTo(0, -size / 2 + h_layer * 2 + 8)
    p3.lineTo(w / 2, -size / 2 + h_layer + 8)
    p.drawPath(p3)
    p.restore()


def draw_delta_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Minimalist delta triangle mark (AVIF)."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w = size * 0.9
    h = size * 0.85

    path = QPainterPath()
    path.moveTo(0, -h / 2)
    path.lineTo(w / 2, h / 2)
    path.lineTo(-w / 2, h / 2)
    path.closeSubpath()
    p.drawPath(path)

    # Inner cut
    inner_path = QPainterPath()
    inner_path.moveTo(0, -h / 6)
    inner_path.lineTo(w / 4, h / 3)
    inner_path.lineTo(-w / 4, h / 3)
    inner_path.closeSubpath()
    p.drawPath(inner_path)
    p.restore()


def draw_video_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Video / media player frame icon."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w, h = size, size * 0.75
    p.drawRoundedRect(QRectF(-w / 2, -h / 2, w, h), 3, 3)

    # Play triangle
    p.setBrush(QBrush(color))
    p.setPen(Qt.PenStyle.NoPen)
    path = QPainterPath()
    path.moveTo(-2, -h / 4)
    path.lineTo(3.5, 0)
    path.lineTo(-2, h / 4)
    path.closeSubpath()
    p.drawPath(path)
    p.restore()


def draw_audio_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Music notes / audio icon."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    # Double eighth note
    path = QPainterPath()
    # Left note head
    path.addEllipse(QPointF(-size / 3, size / 3), 2.8, 2.2)
    # Right note head
    path.addEllipse(QPointF(size / 4, size / 5), 2.8, 2.2)
    p.setBrush(QBrush(color))
    p.drawPath(path)

    # Stems and crossbar
    p.setBrush(Qt.BrushStyle.NoBrush)
    p.drawLine(QPointF(-size / 3 + 2.4, size / 3), QPointF(-size / 3 + 2.4, -size / 3))
    p.drawLine(QPointF(size / 4 + 2.4, size / 5), QPointF(size / 4 + 2.4, -size / 2.3))
    # Bar
    bar_pen = QPen(color, 2.2, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap)
    p.setPen(bar_pen)
    p.drawLine(QPointF(-size / 3 + 2.4, -size / 3), QPointF(size / 4 + 2.4, -size / 2.3))
    p.restore()


def draw_sheet_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Spreadsheet / table icon."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w = size * 0.9
    h = size * 0.8
    p.drawRoundedRect(QRectF(-w / 2, -h / 2, w, h), 2.5, 2.5)

    # Grid lines
    p.drawLine(QPointF(-w / 2, -h / 6), QPointF(w / 2, -h / 6))
    p.drawLine(QPointF(-w / 2, h / 5), QPointF(w / 2, h / 5))
    p.drawLine(QPointF(-w / 6, -h / 2), QPointF(-w / 6, h / 2))
    p.drawLine(QPointF(w / 5, -h / 2), QPointF(w / 5, h / 2))
    p.restore()


def draw_archive_icon(p: QPainter, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Archive / ZIP box icon."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setPen(QPen(color, 1.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin))
    p.setBrush(Qt.BrushStyle.NoBrush)

    w = size
    h = size * 0.8
    p.drawRoundedRect(QRectF(-w / 2, -h / 2, w, h), 3, 3)

    # Horizontal split
    p.drawLine(QPointF(-w / 2, -h / 6), QPointF(w / 2, -h / 6))

    # Zipper teeth pattern
    p.setPen(QPen(color, 1.3))
    p.drawLine(QPointF(-1.5, -h / 2), QPointF(-1.5, -h / 6))
    p.drawLine(QPointF(1.5, -h / 2), QPointF(1.5, -h / 6))
    p.drawRect(QRectF(-2.5, -h / 6, 5, 5))
    p.restore()


def draw_arrow_up_icon(p: QPainter, cx: float, cy: float, size: float = 24, color: QColor = QColor(130, 130, 135)):
    """Sleek upward pointing arrow for center hub."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    pen = QPen(color, 2.0, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin)
    p.setPen(pen)

    h = size * 0.85
    w = size * 0.65

    # Shaft
    p.drawLine(QPointF(0, h / 2), QPointF(0, -h / 2))

    # Chevron head
    path = QPainterPath()
    path.moveTo(-w / 2, -h / 6)
    path.lineTo(0, -h / 2)
    path.lineTo(w / 2, -h / 6)
    p.drawPath(path)
    p.restore()


def draw_checkmark_icon(p: QPainter, cx: float, cy: float, size: float = 32, circle_color: QColor = QColor("#78A942")):
    """Checkmark badge for success state."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)

    # Circle background
    r = size / 2
    p.setBrush(QBrush(circle_color))
    p.setPen(Qt.PenStyle.NoPen)
    p.drawEllipse(QPointF(0, 0), r, r)

    # White checkmark
    pen = QPen(QColor(255, 255, 255), 2.6, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap, Qt.PenJoinStyle.RoundJoin)
    p.setPen(pen)
    path = QPainterPath()
    path.moveTo(-r * 0.42, 0)
    path.lineTo(-r * 0.1, r * 0.35)
    path.lineTo(r * 0.45, -r * 0.32)
    p.drawPath(path)
    p.restore()


def draw_error_icon(p: QPainter, cx: float, cy: float, size: float = 32, circle_color: QColor = QColor("#D93025")):
    """Warning badge for error state."""
    p.save()
    p.translate(cx, cy)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)

    r = size / 2
    p.setBrush(QBrush(circle_color))
    p.setPen(Qt.PenStyle.NoPen)
    p.drawEllipse(QPointF(0, 0), r, r)

    # White exclamation mark
    pen = QPen(QColor(255, 255, 255), 2.5, Qt.PenStyle.SolidLine, Qt.PenCapStyle.RoundCap)
    p.setPen(pen)
    p.drawLine(QPointF(0, -r * 0.45), QPointF(0, r * 0.05))
    p.drawPoint(QPointF(0, r * 0.42))
    p.restore()


def draw_format_icon(p: QPainter, format_name: str, cx: float, cy: float, size: float = 20, color: QColor = QColor(40, 40, 45)):
    """Dispatch icon drawing according to format."""
    fmt = format_name.upper()

    if fmt in ("JPG", "JPEG", "PNG", "WEBP", "BMP", "ICO", "GIF", "SVG"):
        draw_image_icon(p, cx, cy, size, color)
    elif fmt == "HEIC":
        draw_camera_icon(p, cx, cy, size, color)
    elif fmt in ("TIFF", "TIF"):
        draw_layers_icon(p, cx, cy, size, color)
    elif fmt == "AVIF":
        draw_delta_icon(p, cx, cy, size, color)
    elif fmt in ("PDF", "DOCX", "DOC", "TXT", "MD", "HTML", "RTF"):
        draw_doc_icon(p, cx, cy, size, color)
    elif fmt in ("MP4", "MOV", "MKV", "WEBM", "AVI", "WMV"):
        draw_video_icon(p, cx, cy, size, color)
    elif fmt in ("MP3", "WAV", "FLAC", "M4A", "OGG", "AAC", "OPUS", "AIFF"):
        draw_audio_icon(p, cx, cy, size, color)
    elif fmt in ("XLSX", "XLS", "CSV", "TSV", "ODS"):
        draw_sheet_icon(p, cx, cy, size, color)
    elif fmt in ("ZIP", "TAR", "GZ", "7Z", "EXTRACT"):
        draw_archive_icon(p, cx, cy, size, color)
    elif fmt == "ALL":
        draw_star_all_icon(p, cx, cy, size, color)
    else:
        draw_doc_icon(p, cx, cy, size, color)
