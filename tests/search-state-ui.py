"""Search survives toggles and focus interruptions on a private display/profile."""
import importlib
import json
import subprocess
import time
from PIL import Image

Desktop = importlib.import_module('canvas-ui').Desktop
d = Desktop()

def visible_text(name, expected):
    time.sleep(.4)
    path = d.shot(name + '.png')
    crop = path.with_name(name + '-search.png')
    Image.open(path).crop((500, 25, 1420, 150)).resize((2760, 375)).save(crop)
    result = subprocess.check_output(['tesseract', str(crop), 'stdout', '--psm', '6'], stderr=subprocess.DEVNULL).decode()
    assert expected.lower() in result.lower(), (expected, result, path)

def resume():
    d.toggle()
    d.x.XSetInputFocus(d.connection, d.window, 1, 0)
    d.flush()
    time.sleep(.5)

try:
    d.start()
    d.stop()
    fixture = d.data()
    fixture['smooth_cursor'] = False
    d.config.write_text(json.dumps(fixture))
    d.start()
    d.text('OrbitDraft')
    visible_text('draft', 'OrbitDraft')
    d.key('Left')
    d.toggle()
    time.sleep(.3)
    resume()
    visible_text('toggled', 'OrbitDraft')
    d.text('x')
    visible_text('cursor-preserved', 'OrbitDrafxt')
    # Focusing the private root exercises the daemon's focus-loss hide timer.
    d.x.XSetInputFocus(d.connection, d.root, 1, 0)
    d.flush()
    time.sleep(.6)
    resume()
    visible_text('interrupted', 'OrbitDrafxt')
    d.key('Escape')
    visible_text('explicit-clear', 'Search anything')
    d.click(900, 60)
    time.sleep(.3)
    d.text('gh')
    d.key('Tab')
    d.text('orbit')
    visible_text('prefix', 'orbit')
    badge = Image.open(d.work / 'prefix.png').crop((725, 43, 786, 74)).tobytes()
    d.toggle()
    time.sleep(.3)
    resume()
    visible_text('prefix-restored', 'orbit')
    assert Image.open(d.work / 'prefix-restored.png').crop((725, 43, 786, 74)).tobytes() == badge, 'Prefix badge changed'
    print('PASS search draft, cursor, Prefix, toggle, interruption and explicit clear:', d.work)
finally:
    d.close()
