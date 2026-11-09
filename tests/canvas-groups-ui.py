"""Private-display regression check for Canvas selection, Groups and ink. Build release first."""
import sys, importlib, json, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
c = importlib.import_module('canvas-ui')
d = c.Desktop()
try:
    d.start()
    d.stop()
    f = d.data()
    f['items'] = [{'App': dict(id=i, name=i, icon='Letter', launch={'Command': 'true'})} for i in ['Alpha', 'Beta', 'Far']]
    f['positions'] = {i: dict(col=x, row=300, width=144, height=144) for i, x in [('Alpha', 400), ('Beta', 650), ('Far', 2400)]}
    f['canvas'] = {'view': dict(x=0, y=0, zoom=1)}
    d.config.write_text(json.dumps(f))
    d.start()
    d.drag((360, 260), (850, 500))
    d.key('Control', True)
    d.click(722, 372)
    d.click(722, 372)
    d.key('Control', False)
    d.shot('selected.png')
    d.click(900, 600, 3)
    d.shot('menu.png')
    d.click(960, 625)
    d.shot('group-editor.png')
    d.text('Work')
    d.shot('group-name.png')
    d.click(860, 595)
    d.key('Control', True)
    d.key('g')
    d.key('Control', False)
    d.click(915, 690)
    d.wait(lambda: len(d.data().get('groups', [])) == 1, 'group was not saved')
    group = d.data()['groups'][0]
    assert group['name'] == 'Work' and group['keys'] == 'Ctrl+G' and (set(group['items']) == {'Alpha', 'Beta'}), group
    d.shot('group-saved.png')
    before = d.data()['canvas']['view']
    d.key('Control', True)
    d.key('g')
    d.key('Control', False)
    d.wait(lambda: d.data()['canvas']['view'] != before, 'group shortcut did not fit')
    fitted = d.data()['canvas']['view']
    d.shot('group-fit.png')
    d.stop()
    d.start()
    assert d.data()['groups'][0] == group
    assert d.data()['canvas']['view'] == fitted
    d.shot('group-restarted.png')
    d.click(40, 560)
    d.drag((1100, 600), (1260, 670))
    time.sleep(0.6)
    ink = d.data()['ink']
    assert len(ink) == 1, ink
    first = ink[0]['points'][0]
    assert abs(first['x'] * fitted['zoom'] + fitted['x'] - 1100) < 2, first
    assert abs(first['y'] * fitted['zoom'] + fitted['y'] - 600) < 2, first
    path = d.shot('draw-fitted.png')
    from PIL import Image
    pixel = Image.open(path).getpixel((1180, 635))[:3]
    assert min(pixel) > 180, ('stroke is not visible at pointer', pixel)
    d.stop()
    d.start()
    assert len(d.data()['ink']) == 1
    path = d.shot('ink-restarted.png')
    assert min(Image.open(path).getpixel((1180, 635))[:3]) > 180
    d.click(40, 604)
    d.drag((1100, 600), (1260, 670))
    d.wait(lambda: not d.data()['ink'], 'eraser did not remove stroke')
    d.shot('erased-fitted.png')
    d.click(40, 516)
    d.drag((800, 800), (1200, 800))
    d.click(40, 560)
    d.drag((400, 700), (560, 770))
    time.sleep(0.6)
    assert d.data()['ink'][0]['points'][0]['x'] < 0
    path = d.shot('draw-negative.png')
    assert min(Image.open(path).getpixel((480, 735))[:3]) > 180
    d.click(40, 604)
    d.drag((400, 700), (560, 770))
    d.wait(lambda: not d.data()['ink'], 'negative-coordinate stroke was not erased')
    d.click(40, 472)
    d.key('Control', True)
    d.key('g')
    d.key('Control', False)
    d.wait(lambda: d.data()['canvas']['view'] == fitted, 'group did not restore fitted view')
    d.click(1758, 56)
    d.click(40, 560)
    d.drag((1100, 700), (1260, 770))
    time.sleep(0.4)
    assert not d.data()['ink'], 'Lock layout allowed ink'
    d.click(1758, 56)
    d.click(40, 472)
    d.click(620, 353, 3)
    d.shot('group-context.png')
    d.click(690, 420)
    d.shot('group-edit.png')
    d.click(860, 595)
    d.key('Control', True)
    d.key('s')
    d.key('Control', False)
    d.click(915, 690)
    time.sleep(0.5)
    assert d.data()['groups'][0]['keys'] == 'Ctrl+G', 'conflicting shortcut was saved'
    d.shot('group-conflict.png')
    d.click(860, 595)
    d.key('Control', True)
    d.key('g')
    d.key('Control', False)
    d.click(850, 494)
    d.key('Control', True)
    d.key('a')
    d.key('Control', False)
    d.text('Team')
    d.click(915, 690)
    d.wait(lambda: d.data()['groups'][0]['name'] == 'Team', 'edit was not saved')
    d.shot('group-edited.png')
    d.stop()
    fixture = d.data()
    fixture['groups'][0]['name'] = 'عمل Work'
    d.config.write_text(json.dumps(fixture))
    d.start()
    d.key('Control', True)
    d.key('g')
    d.key('Control', False)
    d.shot('group-arabic.png')
    d.click(620, 353, 3)
    d.click(690, 420)
    d.shot('group-arabic-editor.png')
    d.key('Escape')
    d.click(620, 353, 3)
    d.click(690, 460)
    d.wait(lambda: not d.data()['groups'], 'group was not deleted')
    assert len(d.data()['items']) == 3, 'deleting a Group deleted items'
    d.shot('group-deleted.png')
    print(d.work, flush=True)
finally:
    d.close()
