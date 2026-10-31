"""
Gesture & Hotkey Wakeup System for Mossambi.
Features:
1. Global Hotkey: Win + Ctrl to summon the Radial Wheel under the cursor.
2. Circle-Drag Wakeup: Summons the widget automatically when the user moves the mouse
   in a circular motion (clockwise or counter-clockwise, Delta theta >= 300 deg) while dragging a file.
"""

import sys
import math
import time
import threading
from typing import Optional, List, Tuple
from PyQt6.QtCore import QObject, pyqtSignal, QPointF


class WakeupListener(QObject):
    """
    Background daemon listener for Win+Ctrl global hotkey
    and mouse circle-drag gesture detection.
    """
    summon_hotkey = pyqtSignal(int, int)          # (cursor_x, cursor_y)
    summon_circle_gesture = pyqtSignal(int, int)  # (cursor_x, cursor_y)

    def __init__(self, parent: Optional[QObject] = None):
        super().__init__(parent)
        self._running = False
        self._thread: Optional[threading.Thread] = None
        self.enable_gesture = True
        self.enable_hotkey = True

    def start(self):
        if sys.platform != "win32":
            return
        self._running = True
        self._thread = threading.Thread(target=self._worker_loop, daemon=True)
        self._thread.start()

    def stop(self):
        self._running = False
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=0.5)

    def _worker_loop(self):
        import ctypes
        from ctypes import wintypes

        user32 = ctypes.windll.user32

        class POINT(ctypes.Structure):
            _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]

        VK_LBUTTON = 0x01
        VK_LWIN = 0x5B
        VK_RWIN = 0x5C
        VK_CONTROL = 0x11

        hotkey_was_down = False
        is_dragging = False
        drag_points: List[Tuple[int, int, float]] = [] # (x, y, timestamp)
        cumulative_angle = 0.0
        last_angle: Optional[float] = None
        center_x = 0.0
        center_y = 0.0
        last_gesture_trigger_time = 0.0

        pt = POINT()

        while self._running:
            time.sleep(0.016)  # ~60 Hz sampling rate

            # 1. Hotkey Check: Win + Ctrl
            if self.enable_hotkey:
                win_down = (user32.GetAsyncKeyState(VK_LWIN) & 0x8000 != 0) or (user32.GetAsyncKeyState(VK_RWIN) & 0x8000 != 0)
                ctrl_down = (user32.GetAsyncKeyState(VK_CONTROL) & 0x8000 != 0)

                if win_down and ctrl_down:
                    if not hotkey_was_down:
                        hotkey_was_down = True
                        user32.GetCursorPos(ctypes.byref(pt))
                        self.summon_hotkey.emit(pt.x, pt.y)
                else:
                    hotkey_was_down = False

            # 2. Circle-Drag Gesture Check
            if self.enable_gesture:
                lbutton_down = (user32.GetAsyncKeyState(VK_LBUTTON) & 0x8000 != 0)
                now = time.time()

                if lbutton_down:
                    user32.GetCursorPos(ctypes.byref(pt))
                    cx, cy = pt.x, pt.y

                    if not is_dragging:
                        # Drag began
                        is_dragging = True
                        drag_points = [(cx, cy, now)]
                        cumulative_angle = 0.0
                        last_angle = None
                        center_x = float(cx)
                        center_y = float(cy)
                    else:
                        # Append point if mouse moved at least 3 pixels
                        last_x, last_y, _ = drag_points[-1]
                        dist_sq = (cx - last_x) ** 2 + (cy - last_y) ** 2
                        if dist_sq >= 9:
                            drag_points.append((cx, cy, now))

                            # Prune points older than 1.5 seconds
                            drag_points = [p for p in drag_points if now - p[2] <= 1.5]

                            if len(drag_points) >= 6:
                                # Estimate circle center as mean of bounding points
                                xs = [p[0] for p in drag_points]
                                ys = [p[1] for p in drag_points]
                                center_x = sum(xs) / len(xs)
                                center_y = sum(ys) / len(ys)

                                dx = cx - center_x
                                dy = cy - center_y
                                radius = math.hypot(dx, dy)

                                # Radius must be in human circular gesture window [15px, 220px]
                                if 15.0 <= radius <= 220.0:
                                    current_angle = math.atan2(dy, dx)
                                    if last_angle is not None:
                                        delta = current_angle - last_angle
                                        # Normalize delta to [-pi, pi]
                                        while delta > math.pi:
                                            delta -= 2 * math.pi
                                        while delta < -math.pi:
                                            delta += 2 * math.pi

                                        cumulative_angle += delta

                                        # Check if circular rotation >= 300 degrees (5.23 radians)
                                        if abs(cumulative_angle) >= (300.0 * math.pi / 180.0):
                                            if now - last_gesture_trigger_time > 2.0:
                                                last_gesture_trigger_time = now
                                                self.summon_circle_gesture.emit(cx, cy)
                                                # Reset trajectory to avoid immediate retrigger
                                                drag_points.clear()
                                                cumulative_angle = 0.0
                                                last_angle = None

                                    last_angle = current_angle
                else:
                    # Mouse released
                    is_dragging = False
                    drag_points.clear()
                    cumulative_angle = 0.0
                    last_angle = None
