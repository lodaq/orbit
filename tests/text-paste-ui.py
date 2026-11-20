"""Paste real system clipboard text on a private display and profile."""
import importlib
import subprocess

Desktop = importlib.import_module('canvas-ui').Desktop


def chord(d, key):
    d.key('Control', True)
    d.key(key)
    d.key('Control', False)


def annotations(d):
    return [item['Annotation'] for item in d.data()['items'] if 'Annotation' in item]


d = Desktop()
owner = None
try:
    owner = subprocess.Popen(['/usr/bin/python3', '-c', '''
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk
clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
clipboard.set_text('Hello Canvas\\nمرحبا بالعالم', -1)
print('ready', flush=True)
Gtk.main()
'''], env=d.env, stdout=subprocess.PIPE, stderr=d.log, text=True)
    assert owner.stdout.readline().strip() == 'ready'
    d.start()
    d.click(1758, 56)
    d.wait(lambda: d.data()['layout_locked'], 'layout did not lock')
    d.click(40, 340)
    chord(d, 'v')
    assert len(annotations(d)) == 0, 'Lock layout allowed text paste'
    d.click(1758, 56)
    d.wait(lambda: not d.data()['layout_locked'], 'layout did not unlock')
    d.click(40, 340)  # Select gives the Canvas focus.
    chord(d, 'v')
    d.wait(lambda: len(annotations(d)) == 1, 'clipboard text was not pasted')
    image = annotations(d)[0]
    assert image['name'] == 'Hello Canvas\nمرحبا بالعالم', image
    assert image['kind'] == 2, image
    d.shot('text-pasted.png')
    chord(d, 'z')
    d.wait(lambda: len(annotations(d)) == 0, 'Undo did not remove pasted text')
    d.key('Control', True); d.key('Shift', True); d.key('z'); d.key('Shift', False); d.key('Control', False)
    d.wait(lambda: len(annotations(d)) == 1, 'Redo did not restore pasted text')
    d.key('Escape')
    d.click(1400, 200, 3)
    d.click(1460, 384)
    d.wait(lambda: len(annotations(d)) == 2, 'Context menu text Paste failed')
    chord(d, 'z')
    d.wait(lambda: len(annotations(d)) == 1, 'Undo menu text paste failed')
    saved = d.data()
    d.stop(); d.start()
    assert len(annotations(d)) == 1
    assert d.data()['positions'] == saved['positions']
    d.shot('text-paste-restarted.png')
    print('PASS clipboard Text paste, Undo/Redo and restart:', d.work)
finally:
    d.close()
    if owner:
        owner.terminate(); owner.wait(timeout=5)
