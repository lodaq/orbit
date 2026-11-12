"""Canvas editing shortcuts and live pointer feedback on a private display/profile.

Run after cargo build --release. No input reaches the user's desktop.
"""
import importlib
import json
import time

from PIL import Image

Desktop = importlib.import_module('canvas-ui').Desktop


def chord(d, key, shift=False):
    d.key('Control', True)
    if shift:
        d.key('Shift', True)
    d.key(key)
    if shift:
        d.key('Shift', False)
    d.key('Control', False)
    time.sleep(.15)


def white(path, x, y):
    return min(Image.open(path).getpixel((x, y))[:3]) > 180


d = Desktop()
try:
    d.start()
    d.stop()
    fixture = d.data()
    fixture['items'] = [dict(App=dict(id=name, name=name, icon='Letter',
                                    launch=dict(Command='true')))
                        for name in ['Alpha', 'Beta', 'Far']]
    fixture['positions'] = {name: dict(col=x, row=300, width=144, height=144)
                            for name, x in [('Alpha', 400), ('Beta', 650), ('Far', 2400)]}
    fixture['frames'] = [dict(id='work', name='Work', items=['Alpha', 'Beta'], keys='Ctrl+G'),
                         dict(id='far', name='عمل Far', items=['Far'], keys='Ctrl+B')]
    fixture['ink'] = []
    fixture['canvas'] = dict(view=dict(x=0, y=0, zoom=1))
    d.config.write_text(json.dumps(fixture))
    d.start()
    d.click(1500, 800)
    d.wheel(1500, 800, -1)
    d.wait(lambda: d.data()['canvas']['view']['y'] != 0, 'wheel did not pan vertically')
    vertical = d.data()['canvas']['view']
    assert vertical['x'] == 0 and vertical['zoom'] == 1
    d.key('Shift', True)
    d.wheel(1500, 800, -1)
    d.key('Shift', False)
    d.wait(lambda: d.data()['canvas']['view']['x'] != 0, 'Shift+wheel did not pan horizontally')
    horizontal = d.data()['canvas']['view']
    assert horizontal['y'] == vertical['y'] and horizontal['zoom'] == 1
    d.key('Control', True)
    d.wheel(1500, 800, 1)
    d.key('Control', False)
    d.wait(lambda: d.data()['canvas']['view']['zoom'] > 1, 'Ctrl+wheel did not zoom')

    chord(d, 'j')
    d.wait(lambda: d.data()['canvas']['view']['zoom'] == 2, 'first Frame was not fitted')
    work = d.data()['canvas']['view']
    chord(d, 'l')
    d.wait(lambda: d.data()['canvas']['view'] != work, 'Ctrl+L did not fit next Frame')
    far = d.data()['canvas']['view']
    chord(d, 'k')
    d.wait(lambda: d.data()['canvas']['view'] == work, 'Ctrl+K did not fit previous Frame')
    chord(d, 'j')
    d.wait(lambda: d.data()['canvas']['view'] == far, 'Ctrl+J did not fit next Frame')
    chord(d, 'h')
    d.wait(lambda: d.data()['canvas']['view'] == work, 'Ctrl+H did not fit previous Frame')
    d.shot('frames-keyboard.png')

    d.key('1')
    d.key('Control', True)
    d.click(*d.item_point('Alpha'))
    d.click(*d.item_point('Beta'))
    d.key('Control', False)
    chord(d, 'c')
    chord(d, 'v')
    d.wait(lambda: len(d.items()) == 5, 'copy/paste did not duplicate both items')
    assert len({item['id'] for item in d.items()}) == 5
    d.shot('copied.png')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 3, 'Undo did not remove pasted items')
    chord(d, 'z', shift=True)
    d.wait(lambda: len(d.items()) == 5, 'Redo did not restore pasted items')
    d.key('Control', True)
    d.click(*d.item_point('Alpha'))
    d.click(*d.item_point('Beta'))
    d.key('Control', False)
    chord(d, 'x')
    d.wait(lambda: not any(item['id'] == 'Alpha' for item in d.items()), 'Cut did not remove selection')
    chord(d, 'v')
    d.wait(lambda: any(item['id'] == 'Alpha' for item in d.items()), 'Paste lost cut item IDs')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 3, 'Undo paste failed')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 5, 'Undo cut failed')

    d.key('Control', True)
    d.click(*d.item_point('Alpha'))
    d.key('Control', False)
    d.click(1758, 56)
    d.wait(lambda: d.data()['layout_locked'], 'layout did not lock')
    d.click(40, 340)
    locked = d.data()['positions']
    chord(d, 'x')
    chord(d, 'z')
    assert len(d.items()) == 5 and d.data()['positions'] == locked, 'Lock layout allowed an edit'
    chord(d, 'c')
    d.click(1758, 56)
    d.wait(lambda: not d.data()['layout_locked'], 'layout did not unlock')
    d.click(40, 340)
    chord(d, 'v')
    d.wait(lambda: len(d.items()) == 6, 'Copy was blocked by Lock layout')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 5, 'Undo paste after unlocking failed')

    d.key('2')
    before = d.data()['canvas']['view']
    d.drag((1500, 800), (1450, 800))
    d.wait(lambda: d.data()['canvas']['view'] != before, '2 did not select Hand')
    d.key('3')
    d.drag((300, 700), (460, 770), release=False)
    assert not d.data()['ink'], 'Draw saved before release'
    assert white(d.shot('draw-live.png'), 380, 735), 'Draw was not visible while held'
    d.button(1, False)
    d.wait(lambda: len(d.data()['ink']) == 1, 'Draw did not save on release')
    d.key('4')
    d.drag((300, 700), (460, 770), release=False)
    assert len(d.data()['ink']) == 1, 'Eraser saved before release'
    assert not white(d.shot('erase-live.png'), 380, 735), 'Eraser did not update while held'
    d.key('Escape')
    d.button(1, False)
    assert white(d.shot('erase-cancelled.png'), 380, 735), 'Escape did not restore erased preview'
    d.drag((300, 700), (460, 770))
    d.wait(lambda: not d.data()['ink'], 'Eraser did not save')
    chord(d, 'z')
    d.wait(lambda: len(d.data()['ink']) == 1, 'Undo eraser failed')
    chord(d, 'z', shift=True)
    d.wait(lambda: not d.data()['ink'], 'Redo eraser failed')
    d.key('3')
    d.drag((300, 700), (460, 770), release=False)
    d.key('Escape')
    d.button(1, False)
    assert not d.data()['ink'], 'Escape saved the cancelled drawing'
    d.shot('draw-cancelled.png')

    d.drag((300, 700), (460, 770), release=False)
    d.x.XSetInputFocus(d.connection, d.root, 1, 0)
    d.flush()
    time.sleep(.4)
    d.button(1, False)
    if Image.open(d.shot('focus-lost.png')).getpixel((1900, 500))[:3] == (0, 0, 0):
        d.toggle()
        time.sleep(.5)
    d.x.XMapWindow(d.connection, d.window)
    d.x.XSetInputFocus(d.connection, d.window, 1, 0)
    d.flush()
    time.sleep(.2)
    assert not d.data()['ink'], 'Losing window focus saved a cancelled stroke'
    assert not white(d.shot('focus-cancelled.png'), 380, 735)

    d.click(40, 340)
    x, y = d.item_point('Alpha')
    d.click(x, y, 3)
    d.shot('item-edit-menu.png')
    d.click(x + 60, y + 104)
    chord(d, 'v')
    d.wait(lambda: len(d.items()) == 6, 'Context menu Copy failed')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 5, 'Undo context-menu copy/paste failed')
    x, y = d.item_point('Alpha')
    d.click(x, y, 3)
    d.click(x + 60, y + 64)
    d.wait(lambda: len(d.items()) == 4, 'Context menu Cut failed')
    d.click(1400, 200, 3)
    d.shot('canvas-edit-menu.png')
    d.click(1460, 304)
    d.wait(lambda: len(d.items()) == 5, 'Context menu Undo failed')
    d.click(1400, 200, 3)
    d.click(1460, 344)
    d.wait(lambda: len(d.items()) == 4, 'Context menu Redo failed')
    d.click(1400, 200, 3)
    d.click(1460, 384)
    d.wait(lambda: len(d.items()) == 5, 'Context menu Paste failed')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 4, 'Undo mouse paste failed')
    chord(d, 'z')
    d.wait(lambda: len(d.items()) == 5, 'Undo mouse cut failed')

    # Numeric tool keys and edit chords must stay in a focused text field.
    d.click(850, 58)
    d.text('1234')
    chord(d, 'a')
    chord(d, 'c')
    chord(d, 'x')
    chord(d, 'v')
    d.shot('text-editing.png')
    assert len(d.items()) == 5 and not d.data()['ink']
    d.key('Escape')
    d.stop()
    d.start()
    assert len(d.items()) == 5 and not d.data()['ink']
    d.shot('restart.png')
    d.stop()
    fixture = d.data()
    fixture['items'] = fixture['items'][:1]
    item_id = next(iter(fixture['items'][0].values()))['id']
    fixture['positions'] = {item_id: dict(col=400, row=300, width=144, height=144)}
    fixture['frames'] = []
    fixture['canvas'] = dict(view=dict(x=0, y=0, zoom=1))
    fixture['ink'] = [dict(id=name, points=[dict(x=600, y=y), dict(x=750, y=y+30)],
                           style=dict(color='#e8e5ef', size=4, opacity=1,
                                      arrow=name == 'arrow', highlighter=name == 'highlighter'))
                      for name, y in [('pen', 300), ('arrow', 360), ('highlighter', 420)]]
    d.config.write_text(json.dumps(fixture))
    d.start()
    d.click(40, 340)
    d.drag((370, 270), (780, 470))
    d.shot('mixed-selection.png')
    chord(d, 'c')
    chord(d, 'v')
    d.wait(lambda: len(d.data()['ink']) == 6 and len(d.items()) == 2,
           'mixed copy/paste did not duplicate items and strokes')
    d.shot('mixed-paste.png')
    chord(d, 'z')
    d.wait(lambda: len(d.data()['ink']) == 3 and len(d.items()) == 1, 'mixed Undo failed')
    chord(d, 'z', shift=True)
    d.wait(lambda: len(d.data()['ink']) == 6 and len(d.items()) == 2, 'mixed Redo failed')
    d.click(1500, 800)
    d.drag((370, 270), (780, 470))
    chord(d, 'x')
    d.wait(lambda: len(d.data()['ink']) == 3 and len(d.items()) == 1, 'mixed Cut failed')
    chord(d, 'v')
    d.wait(lambda: len(d.data()['ink']) == 6 and len(d.items()) == 2, 'mixed cut Paste failed')
    assert all(any(stroke['id'] == name for stroke in d.data()['ink'])
               for name in ['pen', 'arrow', 'highlighter'])
    d.shot('mixed-cut-paste.png')
    # A stroke can also be selected directly without a rectangle.
    d.click(1500, 800)
    pen = next(stroke for stroke in d.data()['ink'] if stroke['id'] == 'pen')
    point = pen['points'][0]
    d.click(round(point['x'] + 20), round(point['y'] + 4))
    chord(d, 'c')
    chord(d, 'v')
    d.wait(lambda: len(d.data()['ink']) == 7, 'single-stroke click Copy failed')
    d.shot('stroke-click-copy.png')
    print('PASS wheel pan/zoom, Frame navigation, tools, clipboard, undo/redo, live ink and cancellation:', d.work)
finally:
    d.close()
