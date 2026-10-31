"""
Format Registry & Category Classifier for Mossambi.
Defines supported input/output formats and maps files to contextual wheel sectors.
"""

import os
from enum import Enum
from typing import List, Dict, Optional, Tuple


class FileCategory(Enum):
    IMAGE = "image"
    VIDEO = "video"
    AUDIO = "audio"
    DOCUMENT = "document"
    SPREADSHEET = "spreadsheet"
    ARCHIVE = "archive"
    SUBTITLE = "subtitle"
    UNKNOWN = "unknown"


# Mapping from extensions to Category
EXTENSION_CATEGORIES: Dict[str, FileCategory] = {
    # Images
    "JPG": FileCategory.IMAGE,
    "JPEG": FileCategory.IMAGE,
    "PNG": FileCategory.IMAGE,
    "WEBP": FileCategory.IMAGE,
    "HEIC": FileCategory.IMAGE,
    "TIFF": FileCategory.IMAGE,
    "TIF": FileCategory.IMAGE,
    "AVIF": FileCategory.IMAGE,
    "BMP": FileCategory.IMAGE,
    "GIF": FileCategory.IMAGE,
    "ICO": FileCategory.IMAGE,
    "SVG": FileCategory.IMAGE,

    # Video
    "MP4": FileCategory.VIDEO,
    "MOV": FileCategory.VIDEO,
    "MKV": FileCategory.VIDEO,
    "WEBM": FileCategory.VIDEO,
    "AVI": FileCategory.VIDEO,
    "WMV": FileCategory.VIDEO,
    "FLV": FileCategory.VIDEO,

    # Audio
    "MP3": FileCategory.AUDIO,
    "WAV": FileCategory.AUDIO,
    "FLAC": FileCategory.AUDIO,
    "M4A": FileCategory.AUDIO,
    "OGG": FileCategory.AUDIO,
    "AAC": FileCategory.AUDIO,
    "OPUS": FileCategory.AUDIO,
    "AIFF": FileCategory.AUDIO,
    "WMA": FileCategory.AUDIO,

    # Document
    "PDF": FileCategory.DOCUMENT,
    "DOCX": FileCategory.DOCUMENT,
    "DOC": FileCategory.DOCUMENT,
    "TXT": FileCategory.DOCUMENT,
    "MD": FileCategory.DOCUMENT,
    "HTML": FileCategory.DOCUMENT,
    "RTF": FileCategory.DOCUMENT,

    # Spreadsheet
    "XLSX": FileCategory.SPREADSHEET,
    "XLS": FileCategory.SPREADSHEET,
    "CSV": FileCategory.SPREADSHEET,
    "TSV": FileCategory.SPREADSHEET,
    "ODS": FileCategory.SPREADSHEET,

    # Archive
    "ZIP": FileCategory.ARCHIVE,
    "TAR": FileCategory.ARCHIVE,
    "GZ": FileCategory.ARCHIVE,
    "7Z": FileCategory.ARCHIVE,

    # Subtitle
    "SRT": FileCategory.SUBTITLE,
    "VTT": FileCategory.SUBTITLE,
}

# Default idle wheel sectors (8 Petals including golden ALL):
DEFAULT_IDLE_SECTORS: List[str] = [
    "JPG", "PNG", "WEBP", "PDF", "MP4", "MP3", "ZIP", "ALL"
]

# Category to output format mappings (exactly 8 radial sectors ending in golden ALL)
CATEGORY_OUTPUT_FORMATS: Dict[FileCategory, List[str]] = {
    FileCategory.IMAGE: [
        "JPG", "PNG", "WEBP", "PDF", "HEIC", "TIFF", "AVIF", "ALL"
    ],
    FileCategory.VIDEO: [
        "MP4", "MOV", "MKV", "WEBM", "GIF", "MP3", "AVI", "ALL"
    ],
    FileCategory.AUDIO: [
        "MP3", "WAV", "FLAC", "M4A", "OGG", "AAC", "OPUS", "ALL"
    ],
    FileCategory.DOCUMENT: [
        "PDF", "DOCX", "TXT", "HTML", "JPG", "PNG", "EPUB", "ALL"
    ],
    FileCategory.SPREADSHEET: [
        "XLSX", "CSV", "TSV", "PDF", "ODS", "XLS", "HTML", "ALL"
    ],
    FileCategory.ARCHIVE: [
        "ZIP", "7Z", "TAR", "TAR.GZ", "TAR.XZ", "TAR.ZST", "EXTRACT", "ALL"
    ],
    FileCategory.SUBTITLE: [
        "VTT", "SRT", "TXT", "ASS", "SSA", "SUB", "JSON", "ALL"
    ],
    FileCategory.UNKNOWN: [
        "PDF", "ZIP", "PNG", "MP4", "MP3", "TXT", "INFO", "ALL"
    ],
}


def classify_file(filepath: str) -> Tuple[FileCategory, str]:
    """Return category and clean uppercase extension."""
    if os.path.isdir(filepath):
        return FileCategory.ARCHIVE, "DIR"

    _, ext = os.path.splitext(filepath)
    ext_clean = ext.lstrip(".").upper()
    category = EXTENSION_CATEGORIES.get(ext_clean, FileCategory.UNKNOWN)
    return category, ext_clean


def get_available_formats(filepaths: List[str]) -> List[str]:
    """
    Compute valid output format sectors for the given list of files.
    If no files, returns the default idle sectors.
    If multiple files of same category, returns common formats.
    """
    if not filepaths:
        return DEFAULT_IDLE_SECTORS

    categories = set()
    formats_per_file = []

    for path in filepaths:
        cat, ext = classify_file(path)
        categories.add(cat)
        available = list(CATEGORY_OUTPUT_FORMATS.get(cat, DEFAULT_IDLE_SECTORS))
        # Exclude same format if it's the only one
        if ext in available and len(available) > 4:
            # We can still keep it or rotate it
            pass
        formats_per_file.append(available)

    if len(categories) != 1 or FileCategory.UNKNOWN in categories:
        # Mixed categories or unknown: show popular universal formats
        return ["PDF", "ZIP", "COPY", "INFO"]

    # Intersect available formats
    common = formats_per_file[0]
    for other in formats_per_file[1:]:
        common = [f for f in common if f in other]

    return common if common else DEFAULT_IDLE_SECTORS
