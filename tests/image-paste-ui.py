"""Paste a real system clipboard image on a private display and profile."""
import importlib
import subprocess
from PIL import Image

Desktop = importlib.import_module('canvas-ui').Desktop


def chord(d, key):
    d.key('Control', True)
    d.key(key)
    d.key('Control', False)


def images(d):
    return [item['Annotation'] for item in d.data()['items'] if 'Annotation' in item]


d = Desktop()
owner = None
try:
    picture = d.work / 'clipboard.png'
    Image.new('RGBA', (600, 300), (80, 170, 220, 255)).save(picture)
    owner = subprocess.Popen(['/usr/bin/python3', '-c', '''
import gi, sys
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk, GdkPixbuf
clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
clipboard.set_image(GdkPixbuf.Pixbuf.new_from_file(sys.argv[1]))
print('ready', flush=True)
Gtk.main()
''', str(picture)], env=d.env, stdout=subprocess.PIPE, stderr=d.log, text=True)
    assert owner.stdout.readline().strip() == 'ready'
    d.start()
    d.click(1758, 56)
    d.wait(lambda: d.data()['layout_locked'], 'layout did not lock')
    d.click(40, 340)
    chord(d, 'v')
    assert len(images(d)) == 0, 'Lock layout allowed image paste'
    d.click(1758, 56)
    d.wait(lambda: not d.data()['layout_locked'], 'layout did not unlock')
    d.click(40, 340)  # Select gives the Canvas focus.
    chord(d, 'v')
    d.wait(lambda: len(images(d)) == 1, 'clipboard image was not pasted')
    image = images(d)[0]
    position = d.data()['positions'][image['id']]
    assert position['width'] == 400 and position['height'] == 200, position
    def image_visible():
        d.shot('image-pasted.png')
        colors = Image.open(d.work / 'image-pasted.png').convert('RGB').getcolors(1920 * 1080)
        return sum(count for count, color in colors if abs(color[0] - 80) < 20 and abs(color[1] - 170) < 20 and abs(color[2] - 220) < 20) > 40000
    d.wait(image_visible, 'pasted image was not visible')
    chord(d, 'z')
    d.wait(lambda: len(images(d)) == 0, 'Undo did not remove pasted image')
    d.key('Control', True); d.key('Shift', True); d.key('z'); d.key('Shift', False); d.key('Control', False)
    d.wait(lambda: len(images(d)) == 1, 'Redo did not restore pasted image')
    d.key('Escape')
    d.click(1400, 200, 3)
    d.click(1460, 384)
    d.wait(lambda: len(images(d)) == 2, 'Context menu image Paste failed')
    chord(d, 'z')
    d.wait(lambda: len(images(d)) == 1, 'Undo menu image paste failed')
    # Copy must replace the external image, and paste the Canvas selection.
    d.click(40, 340)
    d.click(position['col'] + 100, position['row'] + 80)
    # Select with a rectangle because an unmodified image click only activates it.
    d.drag((position['col'] - 10, position['row'] - 10),
           (position['col'] + 410, position['row'] + 210))
    chord(d, 'c'); chord(d, 'v')
    d.wait(lambda: len(images(d)) == 2, 'Canvas Copy/Paste failed after image paste')
    saved = d.data()
    d.stop(); d.start()
    assert len(images(d)) == 2
    assert d.data()['positions'] == saved['positions']
    d.shot('image-paste-restarted.png')
    print('PASS clipboard Image paste, Undo/Redo, Canvas Copy/Paste and restart:', d.work)
finally:
    d.close()
    if owner:
        owner.terminate(); owner.wait(timeout=5)
