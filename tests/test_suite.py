"""
Automated Comprehensive Test Suite for Mossambi Desktop Widget.
Tests Theme, Registry, Geometry, Conversions, and Widget components.
"""

import os
import sys
import tempfile
import unittest
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import QPointF

# Initialize offscreen Qt app for testing
qapp = QApplication.instance() or QApplication(["mossambi_test", "-platform", "offscreen"])

from src.theme import Theme, ThemeMode
from src.pantone import PantonePalette, PantoneColor
from src.registry import (
    classify_file, get_available_formats, FileCategory, DEFAULT_IDLE_SECTORS, CATEGORY_OUTPUT_FORMATS
)
from src.converter import (
    resolve_sibling_output_path, convert_image, convert_text_to_pdf,
    convert_spreadsheet, convert_archive, convert_subtitles,
    optimize_exam_image, convert_to_all_formats
)
from src.wheel import RadialWheel, WheelState
from src.widget import MossambiWidget
from PIL import Image


class TestMossambiTheme(unittest.TestCase):
    def test_theme_tokens(self):
        theme_light = Theme(ThemeMode.LIGHT, citrus_shade="rawpeel")
        self.assertFalse(theme_light.is_dark)
        self.assertEqual(theme_light.accent.name().upper(), PantonePalette.RAW_PEEL.hex.upper())

        theme_dark = Theme(ThemeMode.DARK, citrus_shade="sweetzest")
        self.assertTrue(theme_dark.is_dark)
        self.assertEqual(theme_dark.accent.name().upper(), PantonePalette.SWEET_ZEST.hex.upper())

    def test_pantone_certified_palette(self):
        self.assertEqual(PantonePalette.RAW_PEEL.hex, "#70C922")
        self.assertEqual(PantonePalette.SWEET_ZEST.hex, "#E4F024")
        self.assertEqual(PantonePalette.RIPE_PULP.hex, "#D1E631")
        self.assertEqual(PantonePalette.CITRUS_GOLD.hex, "#F6B324")
        self.assertEqual(PantonePalette.TANGERINE.hex, "#F88F58")
        self.assertEqual(PantonePalette.DEEP_FOREST.hex, "#2E4D1E")


class TestMossambiRegistry(unittest.TestCase):
    def test_eight_petals_with_golden_all(self):
        for cat, sectors in CATEGORY_OUTPUT_FORMATS.items():
            self.assertEqual(len(sectors), 8, f"Category {cat} must have 8 sectors")
            self.assertEqual(sectors[-1], "ALL", f"Category {cat} sector 8 must be 'ALL'")
    def test_classification(self):
        cat, ext = classify_file("test.png")
        self.assertEqual(cat, FileCategory.IMAGE)
        self.assertEqual(ext, "PNG")

        cat, ext = classify_file("song.mp3")
        self.assertEqual(cat, FileCategory.AUDIO)

        cat, ext = classify_file("video.mp4")
        self.assertEqual(cat, FileCategory.VIDEO)

        cat, ext = classify_file("doc.pdf")
        self.assertEqual(cat, FileCategory.DOCUMENT)

        cat, ext = classify_file("sheet.xlsx")
        self.assertEqual(cat, FileCategory.SPREADSHEET)

    def test_available_formats(self):
        idle_formats = get_available_formats([])
        self.assertEqual(idle_formats, DEFAULT_IDLE_SECTORS)

        img_formats = get_available_formats(["sample.jpg"])
        self.assertIn("PNG", img_formats)
        self.assertIn("WEBP", img_formats)
        self.assertIn("PDF", img_formats)

        audio_formats = get_available_formats(["track.wav"])
        self.assertIn("MP3", audio_formats)
        self.assertIn("FLAC", audio_formats)


class TestMossambiConverter(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()

    def tearDown(self):
        import shutil
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_sibling_path_resolution(self):
        in_file = os.path.join(self.test_dir, "photo.png")
        Path(in_file).touch()

        out_1 = resolve_sibling_output_path(in_file, "jpg")
        expected_1 = os.path.join(self.test_dir, "photo_converted.jpg")
        self.assertEqual(out_1, expected_1)

        # Simulate collision
        Path(out_1).touch()
        out_2 = resolve_sibling_output_path(in_file, "jpg")
        expected_2 = os.path.join(self.test_dir, "photo_converted_1.jpg")
        self.assertEqual(out_2, expected_2)

    def test_image_conversion(self):
        in_file = os.path.join(self.test_dir, "input.png")
        img = Image.new("RGBA", (100, 100), (255, 100, 50, 200))
        img.save(in_file, "PNG")

        out_jpg = os.path.join(self.test_dir, "output.jpg")
        convert_image(in_file, out_jpg, "JPG")
        self.assertTrue(os.path.exists(out_jpg))
        self.assertGreater(os.path.getsize(out_jpg), 0)

        out_webp = os.path.join(self.test_dir, "output.webp")
        convert_image(in_file, out_webp, "WEBP")
        self.assertTrue(os.path.exists(out_webp))

        out_pdf = os.path.join(self.test_dir, "output.pdf")
        convert_image(in_file, out_pdf, "PDF")
        self.assertTrue(os.path.exists(out_pdf))

    def test_text_to_pdf_conversion(self):
        txt_file = os.path.join(self.test_dir, "notes.txt")
        with open(txt_file, "w", encoding="utf-8") as f:
            f.write("Mossambi Native Offline File Converter\nTesting text to PDF.")

        pdf_out = os.path.join(self.test_dir, "notes_converted.pdf")
        convert_text_to_pdf(txt_file, pdf_out)
        self.assertTrue(os.path.exists(pdf_out))
        self.assertGreater(os.path.getsize(pdf_out), 0)

    def test_spreadsheet_conversion(self):
        csv_file = os.path.join(self.test_dir, "data.csv")
        with open(csv_file, "w", encoding="utf-8") as f:
            f.write("Name,Score,Status\nAlice,95,Pass\nBob,88,Pass\n")

        xlsx_out = os.path.join(self.test_dir, "data_converted.xlsx")
        convert_spreadsheet(csv_file, xlsx_out, "XLSX")
        self.assertTrue(os.path.exists(xlsx_out))

        tsv_out = os.path.join(self.test_dir, "data_converted.tsv")
        convert_spreadsheet(xlsx_out, tsv_out, "TSV")
        self.assertTrue(os.path.exists(tsv_out))

    def test_archive_conversion(self):
        sample_file = os.path.join(self.test_dir, "item.txt")
        with open(sample_file, "w") as f:
            f.write("hello")

        zip_out = os.path.join(self.test_dir, "test.zip")
        import zipfile
        with zipfile.ZipFile(zip_out, "w") as zf:
            zf.write(sample_file, "item.txt")

        extract_dir = os.path.join(self.test_dir, "extracted")
        convert_archive(zip_out, extract_dir, "EXTRACT")
        self.assertTrue(os.path.exists(os.path.join(extract_dir, "item.txt")))

    def test_subtitle_conversion(self):
        srt_file = os.path.join(self.test_dir, "subs.srt")
        with open(srt_file, "w", encoding="utf-8") as f:
            f.write("1\n00:00:01,000 --> 00:00:04,000\nHello World!\n")

        vtt_out = os.path.join(self.test_dir, "subs.vtt")
        convert_subtitles(srt_file, vtt_out, "VTT")
        self.assertTrue(os.path.exists(vtt_out))
        with open(vtt_out, "r", encoding="utf-8") as f:
            vtt_content = f.read()
        self.assertIn("WEBVTT", vtt_content)
        self.assertIn("00:00:01.000", vtt_content)


class TestMossambiWidget(unittest.TestCase):
    def test_wheel_polar_hit_testing(self):
        theme = Theme(ThemeMode.LIGHT)
        wheel = RadialWheel(theme)
        wheel.resize(440, 440)

        # Center point is at (220, 220)
        idx, is_hub = wheel._hit_test(QPointF(220, 220))
        self.assertTrue(is_hub)
        self.assertIsNone(idx)

        # Top sector (12 o'clock, e.g. at (220, 90)) -> should hit sector 0
        idx, is_hub = wheel._hit_test(QPointF(220, 90))
        self.assertFalse(is_hub)
        self.assertEqual(idx, 0)

        # Far outside widget -> should return (None, False)
        idx, is_hub = wheel._hit_test(QPointF(10, 10))
        self.assertIsNone(idx)
        self.assertFalse(is_hub)

    def test_widget_creation(self):
        widget = MossambiWidget()
        self.assertEqual(widget.windowTitle(), "Mossambi")
        self.assertIsNotNone(widget.wheel)
        self.assertEqual(len(widget.wheel.sectors), 8)


class TestIndianExamOptimizer(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()

    def tearDown(self):
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_upsc_photo_convergence(self):
        # Create synthetic input image
        src_path = os.path.join(self.temp_dir, "test_input.png")
        img = Image.new("RGB", (800, 1000), color=(140, 180, 220))
        img.save(src_path)

        out_path = os.path.join(self.temp_dir, "upsc_ready.jpg")
        res = optimize_exam_image(
            src_path,
            out_path,
            target_width=350,
            target_height=450,
            dpi=300,
            min_bytes=20 * 1024,
            max_bytes=100 * 1024,
            whiten=True
        )

        self.assertTrue(os.path.exists(res))
        out_size = os.path.getsize(res)
        self.assertLessEqual(out_size, 100 * 1024)

        # Verify exact dimensions and DPI
        res_img = Image.open(res)
        self.assertEqual(res_img.size, (350, 450))
        self.assertEqual(res_img.info.get("dpi"), (300, 300))


class TestCircleGestureMath(unittest.TestCase):
    def test_angular_displacement_full_circle(self):
        import math
        # Generate 16 points tracing a clockwise circle
        points = []
        for i in range(17):
            angle = i * (2 * math.pi / 16.0)
            x = 200 + 80 * math.cos(angle)
            y = 200 + 80 * math.sin(angle)
            points.append((x, y))

        cx, cy = 200.0, 200.0
        cumulative_angle = 0.0
        last_angle = None

        for px, py in points:
            cur_angle = math.atan2(py - cy, px - cx)
            if last_angle is not None:
                delta = cur_angle - last_angle
                while delta > math.pi:
                    delta -= 2 * math.pi
                while delta < -math.pi:
                    delta += 2 * math.pi
                cumulative_angle += delta
            last_angle = cur_angle

        # Should reach approximately 2*pi (360 degrees)
        deg = abs(cumulative_angle) * 180.0 / math.pi
        self.assertGreaterEqual(deg, 300.0)


if __name__ == "__main__":
    unittest.main()
