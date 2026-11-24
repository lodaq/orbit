"""Native Canvas rich text through real input on an isolated display and profile."""
import importlib
import math
import os
from pathlib import Path
import subprocess
import time

canvas = importlib.import_module('canvas-ui')
if os.environ.get('ORBIT_TEXT_TEST_BINARY'):
    canvas.BINARY = Path(os.environ['ORBIT_TEXT_TEST_BINARY'])


def chord(d, key, shift=False):
    d.key('Control', True)
    if shift:
        d.key('Shift', True)
    d.key(key)
    if shift:
        d.key('Shift', False)
    d.key('Control', False)


def text_item(d):
    return next(i for i in d.items() if i['item_type'] == 'Annotation')


def bounds(d):
    item = text_item(d)
    rect = d.data()['positions'][item['id']]
    view = d.data()['canvas']['view']
    gap = d.data()['grid_gap']
    return (view['x'] + (rect['col'] + gap / 2) * view['zoom'],
            view['y'] + (rect['row'] + gap / 2) * view['zoom'],
            (rect['width'] - gap) * view['zoom'],
            (rect['height'] - gap) * view['zoom'])


def select(d):
    x, y, w, h = bounds(d)
    d.click(x + w / 2, y + h / 2)


d = canvas.Desktop()
try:
    print('ARTIFACTS', d.work, flush=True)
    d.start()
    d.click(450, 400)
    d.text('Hello ')
    chord(d, 'b')
    d.text('bold')
    chord(d, 'b')
    d.text(' text grows to fit the words that we type')
    d.key('Return')
    d.text('Second line')
    d.shot('editing.png')
    chord(d, 'Return')
    d.wait(lambda: len(d.items()) == 1, 'Text was not saved')
    item = text_item(d)
    assert item['name'] == 'Hello bold text grows to fit the words that we type\nSecond line', item
    assert any(s['text'] == 'bold' and s['format']['bold'] for s in item['rich_text']['spans']), item
    assert item['rich_text']['spans'][0]['format']['font'] == 3
    before = d.data()['positions'][item['id']]
    d.shot('saved.png')

    select(d)
    x, y, w, h = bounds(d)
    d.shot('selected.png')
    d.drag((x+w, y+h), (x+w+120, y+h+40))
    d.wait(lambda: text_item(d)['rich_text']['scale'] > 1.05, 'Corner scaling was not saved')
    scaled = text_item(d)['rich_text']['scale']
    d.shot('scaled.png')

    x, y, w, h = bounds(d)
    d.drag((x+w, y+h/2), (x+w*.55, y+h/2))
    d.wait(lambda: text_item(d)['rich_text']['width'] > 0, 'Side resize did not set wrapping width')
    assert abs(text_item(d)['rich_text']['scale'] - scaled) < .01
    assert bounds(d)[3] > h
    d.shot('wrapped.png')

    x, y, w, h = bounds(d)
    cx, cy = x+w/2, y+h/2
    sx, sy = x+w+16, y+h+16
    angle = math.radians(20)
    ex = cx+(sx-cx)*math.cos(angle)-(sy-cy)*math.sin(angle)
    ey = cy+(sx-cx)*math.sin(angle)+(sy-cy)*math.cos(angle)
    d.drag((sx, sy), (ex, ey))
    d.wait(lambda: abs(text_item(d)['rich_text']['rotation']) > 10, 'Rotation was not saved')
    d.shot('rotated.png')
    rotated = d.data()
    rotated_document = text_item(d)['rich_text']

    chord(d, 'z')
    d.wait(lambda: abs(text_item(d)['rich_text']['rotation']) < 1, 'Canvas Undo did not restore rotation')
    chord(d, 'z', True)
    d.wait(lambda: abs(text_item(d)['rich_text']['rotation']) > 10, 'Canvas Redo did not restore rotation')
    d.stop()
    d.start()
    assert text_item(d)['rich_text'] == rotated_document
    assert d.data()['positions'] == rotated['positions']
    assert abs(text_item(d)['rich_text']['rotation']) > 10
    d.shot('restarted.png')

    select(d)
    d.key('Return')
    chord(d, 'a')
    d.text('First')
    chord(d, '8', True)
    d.key('Return')
    d.text('Second')
    d.key('Tab')
    d.key('Return')
    d.text('Nested')
    d.shot('nested-list.png')
    chord(d, 'Return')
    d.wait(lambda: 'Nested' in text_item(d)['name'], 'List editing was not saved')
    assert '• First\n  • Second\n  • Nested' == text_item(d)['name'], text_item(d)

    select(d)
    d.key('Return')
    chord(d, 'a')
    owner = subprocess.Popen(['/usr/bin/python3', '-c', '''
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk
clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
clipboard.set_text('مرحبا Orbit 123', -1)
print('ready', flush=True)
Gtk.main()
'''], env=d.env, stdout=subprocess.PIPE, stderr=d.log, text=True)
    try:
        assert owner.stdout.readline().strip() == 'ready'
        chord(d, 'v')
        chord(d, 'i')
        d.shot('mixed-text.png')
        chord(d, 'Return')
        d.wait(lambda: text_item(d)['name'] == 'مرحبا Orbit 123', 'Mixed text was not saved')
    finally:
        owner.terminate()
        owner.wait(timeout=5)

    d.click(1758, 56)
    d.wait(lambda: d.data()['layout_locked'], 'Layout did not lock')
    locked = d.data()
    select(d)
    x, y, w, h = bounds(d)
    d.drag((x+w, y+h), (x+w+80, y+h+60))
    assert d.data() == locked, 'Locked Text was changed'
    d.shot('locked.png')
    print('PASS native formatting, wrapping, scaling, rotation, lists, Arabic, Undo and restart', flush=True)
finally:
    d.close()

# Formatting controls and a native clipboard round-trip from a fresh Canvas.
d = canvas.Desktop()
try:
    print('FORMATTING ARTIFACTS', d.work, flush=True)
    d.start()
    d.click(450, 500)
    d.text('Linked words')
    chord(d, 'a')
    chord(d, 'k')
    d.click(550, 425)
    d.text('https://example.com')
    d.click(1090, 425)
    d.shot('link-applied.png')
    chord(d, 'c')
    d.text('replacement')
    chord(d, 'a')
    chord(d, 'v')
    chord(d, 'Return')
    d.wait(lambda: len(d.items()) == 1, 'Linked Text was not saved')
    item = text_item(d)
    assert item['name'] == 'Linked words', item
    assert all(s['format']['link'] == 'https://example.com' for s in item['rich_text']['spans']), item
    select(d)
    x, y, _, _ = bounds(d)
    toolbar_x = min(max(12, x), 1920-760-12)
    toolbar_y = max(100, y-108)
    d.click(toolbar_x+85, toolbar_y+64)  # Serif on the formatting toolbar.
    d.click(1400, 700)
    d.wait(lambda: all(s['format']['font'] == 1 for s in text_item(d)['rich_text']['spans']), 'Font control was not saved')
    d.shot('serif.png')
    print('PASS Link, rich clipboard and mouse font control', flush=True)
finally:
    d.close()
