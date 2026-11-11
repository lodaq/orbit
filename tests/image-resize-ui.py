"""Canvas Image resize through the real handle on a private display and profile.

Build with cargo build --release, then run python3 tests/image-resize-ui.py.
Screenshots and the isolated profile remain under target/ui-check/canvas-*.
No input or IPC is sent to the user's compositor or resident Orbit daemon.
"""
import importlib
import json
import subprocess
import time

from PIL import Image

Desktop = importlib.import_module('canvas-ui').Desktop

BLUE = (80, 170, 220)


def chord(d, key):
    d.key('Control', True)
    d.key(key)
    d.key('Control', False)


def tool(d, index):
    d.click(40, 340 + index * 44)


def annotations(d):
    return [item['Annotation'] for item in d.data()['items'] if 'Annotation' in item]


def rect(d, image):
    positions = d.data()['positions']
    return positions[image['id']]


def blue_pixels(path):
    picture = Image.open(path).convert('RGB')
    return sum(1 for pixel in picture.getdata() if pixel == BLUE)


d = Desktop()
services = []
bus_pid = None
try:
    bus = subprocess.check_output(['dbus-daemon', '--session', '--fork', '--print-address=1', '--print-pid=1'],
                                  env=d.env, text=True).splitlines()
    d.env.update(DBUS_SESSION_BUS_ADDRESS=bus[0], XDG_CURRENT_DESKTOP='GNOME', XDG_DATA_HOME=str(d.work / 'data'))
    bus_pid = int(bus[1]); (d.work / 'data').mkdir()
    portal = d.work / 'config/xdg-desktop-portal/portals.conf'; portal.parent.mkdir(parents=True)
    portal.write_text('[preferred]\ndefault=gtk\n')
    for program in ['xdg-desktop-portal-gtk', 'xdg-desktop-portal']:
        services.append(subprocess.Popen(['/usr/lib/' + program], env=d.env, stdout=d.log, stderr=d.log))
    time.sleep(1)
    d.start(); print('ARTIFACTS', d.work, flush=True)

    # A Canvas Image is imported through the same native chooser as the real user.
    image_path = d.work / 'test-image.png'
    Image.new('RGB', (600, 300), BLUE).save(image_path)
    tool(d, 6); d.click(1000, 350); time.sleep(1)
    d.shot('image-chooser.png'); chord(d, 'l'); d.text(str(image_path)); d.key('Return'); time.sleep(1)
    d.x.XSetInputFocus(d.connection, d.window, 1, 0); d.flush()
    d.wait(lambda: len(annotations(d)) == 1, 'image was not imported')
    image = annotations(d)[0]
    assert image['kind'] == 3
    before = rect(d, image)
    assert before['width'] == 400 and before['height'] == 200, before
    d.shot('image-imported.png')
    small = blue_pixels(d.work / 'image-imported.png')

    # Clicking the picture leaves the launcher visible and does not launch anything.
    d.click(before['col'] + before['width'] / 2, before['row'] + before['height'] / 2)
    d.shot('image-clicked.png')
    assert blue_pixels(d.work / 'image-clicked.png') == small, 'clicking the Image hid or changed it'

    # The handle sits in the last corner of the Image; dragging it scales the Image.
    gap = d.data()['grid_gap']
    left = before['col'] + gap / 2
    top = before['row'] + gap / 2
    right = left + before['width'] - gap
    bottom = top + before['height'] - gap
    d.drag((right - 16, bottom - 16), (right + 184, bottom - 16))
    d.wait(lambda: rect(d, image)['width'] != before['width'], 'image was not resized')
    after = rect(d, image)
    assert after['width'] == 600 and after['height'] == 300, after
    assert (after['col'], after['row']) == (before['col'], before['row'])
    d.shot('image-resized.png')
    large = blue_pixels(d.work / 'image-resized.png')
    assert large > small * 2, 'the Image did not grow on screen: %s -> %s' % (small, large)

    # The new size survives a normal daemon restart.
    d.stop(); d.start()
    assert rect(d, image) == after
    d.shot('image-restarted.png')
    print('PASS Canvas Image resize keeps its proportions:', d.work, flush=True)
finally:
    d.close()
    for service in reversed(services):
        service.terminate()
        try:
            service.wait(timeout=5)
        except subprocess.TimeoutExpired:
            service.kill()
    if bus_pid: __import__('os').kill(bus_pid, 15)
