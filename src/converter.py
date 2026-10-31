"""
Mossambi Offline Conversion Engine.
Processes Image, Audio, Video, Document, Spreadsheet, and Archive conversions locally.
Adheres to the non-destructive sibling output rule.
"""

import os
import re
import csv
import shutil
import zipfile
import tarfile
import subprocess
from pathlib import Path
from typing import Optional, List, Tuple, Callable
from PyQt6.QtCore import QThread, pyqtSignal

from PIL import Image
import pypdf


def resolve_sibling_output_path(
    input_path: str,
    target_ext: str,
    output_dir: Optional[str] = None
) -> str:
    """
    Computes a non-destructive output sibling path.
    Appends an incremental suffix (_1, _2, etc.) if a collision occurs.
    """
    p = Path(input_path)
    base_dir = Path(output_dir) if output_dir and os.path.exists(output_dir) else p.parent
    stem = p.stem
    clean_ext = target_ext.lstrip(".").lower()

    # Base candidate: <stem>_converted.<ext>
    candidate_stem = f"{stem}_converted"
    candidate = base_dir / f"{candidate_stem}.{clean_ext}"

    if not candidate.exists():
        return str(candidate)

    # Collision loop: append _1, _2, etc.
    counter = 1
    while True:
        candidate = base_dir / f"{candidate_stem}_{counter}.{clean_ext}"
        if not candidate.exists():
            return str(candidate)
        counter += 1


from .registry import classify_file, FileCategory, CATEGORY_OUTPUT_FORMATS


def optimize_exam_image(
    input_path: str,
    output_path: str,
    target_width: int = 350,
    target_height: int = 450,
    dpi: int = 300,
    min_bytes: int = 20 * 1024,
    max_bytes: int = 100 * 1024,
    whiten: bool = True
) -> str:
    """
    Indian Competitive Exam & Job Application Form Tuner.
    Applies exact DPI, pixel resizing, background whitening for mobile scans,
    and adaptive binary-search JPEG quality convergence to hit the [min_bytes, max_bytes] window.
    """
    from PIL import ImageEnhance, ImageOps
    import io

    img = Image.open(input_path)
    if whiten:
        img = ImageOps.autocontrast(img, cutoff=2)
        enhancer = ImageEnhance.Contrast(img)
        img = enhancer.enhance(1.15)

    img = img.convert("RGB")
    resized = img.resize((target_width, target_height), Image.Resampling.LANCZOS)

    low_q = 5
    high_q = 95
    best_buf = None
    best_q = 85

    for _ in range(7):
        mid_q = (low_q + high_q) // 2
        buf = io.BytesIO()
        resized.save(buf, format="JPEG", quality=mid_q, dpi=(dpi, dpi), optimize=True)
        size = buf.tell()

        if min_bytes <= size <= max_bytes:
            best_buf = buf
            best_q = mid_q
            break
        elif size < min_bytes:
            best_buf = buf
            low_q = mid_q + 1
        else:
            best_buf = buf
            high_q = mid_q - 1

    if best_buf is None:
        best_buf = io.BytesIO()
        resized.save(best_buf, format="JPEG", quality=best_q, dpi=(dpi, dpi), optimize=True)

    with open(output_path, "wb") as f:
        f.write(best_buf.getvalue())

    return output_path


def convert_to_all_formats(
    input_path: str,
    output_dir: Optional[str] = None,
    quality: int = 92
) -> List[str]:
    """Batch convert input file to ALL valid formats in its category simultaneously."""
    category, src_ext = classify_file(input_path)
    all_formats = CATEGORY_OUTPUT_FORMATS.get(category, ["PNG", "JPG", "WEBP", "PDF"])
    valid_targets = [f for f in all_formats if f != "ALL" and f != "EXTRACT" and f != src_ext]

    outputs = []
    for tgt in valid_targets:
        out_path = resolve_sibling_output_path(input_path, tgt.lower(), output_dir)
        try:
            if category == FileCategory.IMAGE:
                convert_image(input_path, out_path, tgt, quality)
            elif category in (FileCategory.AUDIO, FileCategory.VIDEO):
                convert_media_ffmpeg(input_path, out_path, tgt)
            elif category == FileCategory.SPREADSHEET:
                convert_spreadsheet(input_path, out_path, tgt)
            else:
                convert_image(input_path, out_path, tgt, quality)
            if os.path.exists(out_path):
                outputs.append(out_path)
        except Exception:
            continue

    return outputs


def convert_image(
    input_path: str,
    output_path: str,
    target_format: str,
    quality: int = 92
) -> None:
    """Convert image formats via Pillow."""
    target_fmt = target_format.upper()
    img = Image.open(input_path)

    # Handle Alpha channel for formats that don't support RGBA (JPEG, BMP)
    if target_fmt in ("JPG", "JPEG", "BMP") and img.mode in ("RGBA", "LA", "P"):
        background = Image.new("RGB", img.size, (255, 255, 255))
        if img.mode == "P":
            img = img.convert("RGBA")
        background.paste(img, mask=img.split()[-1] if "A" in img.mode else None)
        img = background
    elif target_fmt == "PNG" and img.mode not in ("RGB", "RGBA", "L"):
        img = img.convert("RGBA")

    # Format specific saves
    if target_fmt in ("JPG", "JPEG"):
        img.save(output_path, "JPEG", quality=quality, optimize=True)
    elif target_fmt == "PNG":
        img.save(output_path, "PNG", optimize=True)
    elif target_fmt == "WEBP":
        img.save(output_path, "WEBP", quality=quality)
    elif target_fmt == "BMP":
        img.save(output_path, "BMP")
    elif target_fmt in ("TIFF", "TIF"):
        img.save(output_path, "TIFF", compression="tiff_lzw")
    elif target_fmt == "ICO":
        img.save(output_path, "ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    elif target_fmt == "GIF":
        img.save(output_path, "GIF")
    elif target_fmt == "PDF":
        if img.mode != "RGB":
            img = img.convert("RGB")
        img.save(output_path, "PDF", resolution=100.0)
    else:
        # Fallback to standard save
        img.save(output_path)


def convert_media_ffmpeg(
    input_path: str,
    output_path: str,
    target_format: str,
    on_progress: Optional[Callable[[int], None]] = None
) -> None:
    """Convert audio or video using system FFmpeg binary."""
    target_fmt = target_format.upper()
    cmd = ["ffmpeg", "-y", "-i", input_path]

    if target_fmt == "GIF":
        # High quality palette-based GIF conversion
        filter_str = "fps=15,scale=640:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
        cmd.extend(["-vf", filter_str])
    elif target_fmt == "MP3":
        # Audio extraction or transcoding to MP3
        cmd.extend(["-vn", "-acodec", "libmp3lame", "-ab", "256k"])
    elif target_fmt == "WAV":
        cmd.extend(["-vn", "-acodec", "pcm_s16le"])
    elif target_fmt == "FLAC":
        cmd.extend(["-vn", "-acodec", "flac"])
    elif target_fmt in ("M4A", "AAC"):
        cmd.extend(["-vn", "-acodec", "aac", "-b:a", "256k"])
    elif target_fmt == "OGG":
        cmd.extend(["-vn", "-acodec", "libvorbis", "-q:a", "6"])
    elif target_fmt == "MP4":
        cmd.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "22", "-c:a", "aac", "-pix_fmt", "yuv420p"])
    elif target_fmt == "WEBM":
        cmd.extend(["-c:v", "libvpx-vp9", "-crf", "30", "-b:v", "0", "-c:a", "libopus"])

    cmd.append(output_path)

    # Execute ffmpeg
    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    )
    _, stderr = process.communicate()

    if process.returncode != 0:
        err_msg = stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"FFmpeg failed with code {process.returncode}: {err_msg[:300]}")


def convert_pdf_to_images(input_path: str, output_path: str, target_format: str = "PNG") -> None:
    """Extract embedded images or pages from PDF using pypdf & Pillow."""
    reader = pypdf.PdfReader(input_path)
    count = 0
    clean_fmt = target_format.upper()

    for idx, page in enumerate(reader.pages):
        for img_obj in page.images:
            try:
                img_data = img_obj.data
                from io import BytesIO
                pil_img = Image.open(BytesIO(img_data))
                if count == 0:
                    out = output_path
                else:
                    p = Path(output_path)
                    out = str(p.parent / f"{p.stem}_{count + 1}{p.suffix}")
                if clean_fmt in ("JPG", "JPEG") and pil_img.mode != "RGB":
                    pil_img = pil_img.convert("RGB")
                pil_img.save(out, "JPEG" if clean_fmt in ("JPG", "JPEG") else "PNG")
                count += 1
            except Exception:
                continue

    if count == 0:
        raise RuntimeError("No raster images could be extracted from this PDF.")


def convert_text_to_pdf(input_path: str, output_path: str) -> None:
    """Generate a clean styled PDF from text/markdown file."""
    with open(input_path, "r", encoding="utf-8", errors="replace") as f:
        text_content = f.read()

    # Use Pillow to create a clean letter-sized document page
    page_w, page_h = 1700, 2200  # High-res 200 DPI Letter
    img = Image.new("RGB", (page_w, page_h), (255, 255, 255))
    from PIL import ImageDraw, ImageFont
    draw = ImageDraw.Draw(img)

    margin_x = 120
    margin_y = 120
    y = margin_y

    font = ImageFont.load_default()

    for line in text_content.splitlines():
        if y > page_h - margin_y:
            break
        draw.text((margin_x, y), line[:120], fill=(20, 20, 20), font=font)
        y += 24

    img.save(output_path, "PDF", resolution=150.0)


def convert_spreadsheet(input_path: str, output_path: str, target_format: str) -> None:
    """Convert between CSV, TSV, and XLSX."""
    target_fmt = target_format.upper()
    _, in_ext = os.path.splitext(input_path)
    in_ext = in_ext.lstrip(".").upper()

    # Reading data into list of rows
    rows: List[List[str]] = []
    if in_ext in ("CSV", "TSV"):
        delim = "\t" if in_ext == "TSV" else ","
        with open(input_path, "r", encoding="utf-8", errors="replace", newline="") as f:
            reader = csv.reader(f, delimiter=delim)
            rows = list(reader)
    elif in_ext in ("XLSX", "XLS"):
        import openpyxl
        wb = openpyxl.load_workbook(input_path, data_only=True)
        sheet = wb.active
        if sheet:
            for r in sheet.iter_rows(values_only=True):
                rows.append([str(c) if c is not None else "" for c in r])

    # Writing to target format
    if target_fmt in ("CSV", "TSV"):
        delim = "\t" if target_fmt == "TSV" else ","
        with open(output_path, "w", encoding="utf-8", newline="") as f:
            writer = csv.writer(f, delimiter=delim)
            writer.writerows(rows)
    elif target_fmt == "XLSX":
        import openpyxl
        wb = openpyxl.Workbook()
        ws = wb.active
        for row in rows:
            ws.append(row)
        wb.save(output_path)


def convert_archive(input_path: str, output_path: str, target_format: str) -> None:
    """Repack archives (ZIP <-> TAR) or extract safely."""
    target_fmt = target_format.upper()

    if target_fmt == "EXTRACT":
        extract_dir = output_path
        os.makedirs(extract_dir, exist_ok=True)
        if zipfile.is_zipfile(input_path):
            with zipfile.ZipFile(input_path, "r") as zf:
                # Zip-Slip protection
                for member in zf.namelist():
                    norm_path = os.path.normpath(os.path.join(extract_dir, member))
                    if not norm_path.startswith(os.path.abspath(extract_dir)):
                        raise RuntimeError(f"Zip-slip directory traversal attempt blocked: {member}")
                zf.extractall(extract_dir)
        elif tarfile.is_tarfile(input_path):
            with tarfile.open(input_path, "r") as tf:
                tf.extractall(extract_dir)
    elif target_fmt == "ZIP":
        if os.path.isdir(input_path):
            shutil.make_archive(output_path.rstrip(".zip"), "zip", input_path)
        elif tarfile.is_tarfile(input_path):
            with tarfile.open(input_path, "r") as tf:
                with zipfile.ZipFile(output_path, "w", zipfile.ZIP_DEFLATED) as zf:
                    for member in tf.getmembers():
                        if member.isfile():
                            f = tf.extractfile(member)
                            if f:
                                zf.writestr(member.name, f.read())
    elif target_fmt == "TAR":
        if zipfile.is_zipfile(input_path):
            with zipfile.ZipFile(input_path, "r") as zf:
                with tarfile.open(output_path, "w") as tf:
                    for name in zf.namelist():
                        data = zf.read(name)
                        from io import BytesIO
                        ti = tarfile.TarInfo(name=name)
                        ti.size = len(data)
                        tf.addfile(ti, BytesIO(data))


def convert_subtitles(input_path: str, output_path: str, target_format: str) -> None:
    """SRT <-> VTT <-> TXT conversion."""
    with open(input_path, "r", encoding="utf-8", errors="replace") as f:
        content = f.read()

    target_fmt = target_format.upper()
    _, in_ext = os.path.splitext(input_path)
    in_ext = in_ext.lstrip(".").upper()

    if in_ext == "SRT" and target_fmt == "VTT":
        # Replace 00:00:00,000 -> 00:00:00.000 and add WEBVTT header
        re_time = re.compile(r"(\d{2}:\d{2}:\d{2}),(\d{3})")
        converted = "WEBVTT\n\n" + re_time.sub(r"\1.\2", content)
    elif in_ext == "VTT" and target_fmt == "SRT":
        body = content.replace("WEBVTT", "").strip()
        re_time = re.compile(r"(\d{2}:\d{2}:\d{2})\.(\d{3})")
        converted = re_time.sub(r"\1,\2", body)
    else:  # To TXT
        time_re = re.compile(r"(?m)^\d+\r?$|^\d{2}:\d{2}:\d{2}[,\.]\d{3}.*|^WEBVTT.*")
        lines = [line.strip() for line in time_re.sub("", content).splitlines() if line.strip()]
        converted = "\n".join(lines)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(converted)


class ConversionWorker(QThread):
    """Asynchronous background worker to execute file conversions without blocking UI."""

    progress_changed = pyqtSignal(int)
    conversion_finished = pyqtSignal(bool, str, str)  # success, output_path, error_message

    def __init__(
        self,
        filepaths: List[str],
        target_format: str,
        output_dir: Optional[str] = None,
        quality: int = 92
    ):
        super().__init__()
        self.filepaths = filepaths
        self.target_format = target_format.upper()
        self.output_dir = output_dir
        self.quality = quality

    def run(self):
        try:
            self.progress_changed.emit(10)
            total = len(self.filepaths)
            last_output = ""

            for idx, filepath in enumerate(self.filepaths):
                target_ext = self.target_format.lower()
                if self.target_format == "EXTRACT":
                    target_ext = "extracted"
                    output_path = resolve_sibling_output_path(filepath, "dir", self.output_dir)
                    output_path = output_path.replace(".dir", "_extracted")
                else:
                    output_path = resolve_sibling_output_path(filepath, target_ext, self.output_dir)

                _, ext = os.path.splitext(filepath)
                ext = ext.lstrip(".").upper()

                self.progress_changed.emit(int(20 + (idx / total) * 60))

                # Special Golden "ALL" Sector: Transcode to ALL formats in domain
                if self.target_format == "ALL":
                    batch_outputs = convert_to_all_formats(filepath, self.output_dir, self.quality)
                    if not batch_outputs:
                        raise RuntimeError(f"No valid target formats available for {ext}")
                    last_output = f"Generated {len(batch_outputs)} files: " + ", ".join(os.path.basename(p) for p in batch_outputs[:3])
                    continue

                # Route according to extension and target
                if ext in ("JPG", "JPEG", "PNG", "WEBP", "BMP", "TIFF", "TIF", "ICO", "GIF", "AVIF", "HEIC"):
                    convert_image(filepath, output_path, self.target_format, self.quality)
                elif ext in ("MP4", "MOV", "MKV", "WEBM", "AVI", "WMV", "MP3", "WAV", "FLAC", "M4A", "OGG", "AAC"):
                    convert_media_ffmpeg(filepath, output_path, self.target_format)
                elif ext == "PDF" and self.target_format in ("PNG", "JPG", "JPEG"):
                    convert_pdf_to_images(filepath, output_path, self.target_format)
                elif ext in ("TXT", "MD") and self.target_format == "PDF":
                    convert_text_to_pdf(filepath, output_path)
                elif ext in ("CSV", "TSV", "XLSX", "XLS") and self.target_format in ("CSV", "TSV", "XLSX"):
                    convert_spreadsheet(filepath, output_path, self.target_format)
                elif ext in ("ZIP", "TAR", "GZ") or self.target_format in ("ZIP", "TAR", "EXTRACT"):
                    convert_archive(filepath, output_path, self.target_format)
                elif ext in ("SRT", "VTT"):
                    convert_subtitles(filepath, output_path, self.target_format)
                else:
                    # Generic image fallback
                    try:
                        convert_image(filepath, output_path, self.target_format, self.quality)
                    except Exception:
                        raise RuntimeError(f"Unsupported conversion: from {ext} to {self.target_format}")

                last_output = output_path

            self.progress_changed.emit(100)
            self.conversion_finished.emit(True, last_output, "")
        except Exception as e:
            self.progress_changed.emit(0)
            self.conversion_finished.emit(False, "", str(e))
