"""Canvas tools through real controls on a private display, profile and chooser bus."""
import ctypes as C
import importlib
import json
import os
import signal
import subprocess
import time
from PIL import Image

Desktop = importlib.import_module('canvas-ui').Desktop

def chord(d, key, shift=False):
    d.key('Control', True)
    if shift: d.key('Shift', True)
    d.key(key)
    if shift: d.key('Shift', False)
    d.key('Control', False)

def tool(d, index):
    d.click(40, 340 + index * 44)

def annotations(d):
    return [item['Annotation'] for item in d.data()['items'] if 'Annotation' in item]

def create_text(d, mode, text, position):
    tool(d, mode)
    d.click(*position)
    d.text(text)
    d.shot('text-editor.png')
    d.click(1130, 696)
    d.wait(lambda: any(a['name'] == text for a in annotations(d)), 'text was not saved')

d = Desktop()
services = []
bus_pid = None
try:
    bus = subprocess.check_output(['dbus-daemon','--session','--fork','--print-address=1','--print-pid=1'], env=d.env, text=True).splitlines()
    d.env.update(DBUS_SESSION_BUS_ADDRESS=bus[0], XDG_CURRENT_DESKTOP='GNOME', XDG_DATA_HOME=str(d.work/'data'))
    bus_pid = int(bus[1]); (d.work/'data').mkdir()
    portal = d.work/'config/xdg-desktop-portal/portals.conf'; portal.parent.mkdir(parents=True)
    portal.write_text('[preferred]\ndefault=gtk\n')
    for program in ['xdg-desktop-portal-gtk','xdg-desktop-portal']:
        services.append(subprocess.Popen(['/usr/lib/'+program],env=d.env,stdout=d.log,stderr=d.log))
    time.sleep(1)
    d.start(); print('ARTIFACTS', d.work, flush=True)
    create_text(d,4,'A saved note',(400,300))
    create_text(d,5,'Canvas text',(800,300))
    d.shot('note-and-text.png')
    # Draw, arrow and highlighter retain distinct styles and geometry.
    for index in [2,7,8]:
        tool(d,index); d.drag((400,650 + index*12),(650,700 + index*12))
    d.wait(lambda: len(d.data()['ink']) == 3, 'drawing tools were not saved')
    assert d.data()['ink'][1]['style']['arrow']
    assert d.data()['ink'][2]['style']['highlighter']
    before = d.data(); tool(d,9); d.drag((900,650),(1200,700))
    d.shot('laser.png'); time.sleep(2)
    assert d.data() == before, 'laser was saved'
    d.shot('laser-cleared.png')
    # Eraser keeps saved content until release; Escape restores the held preview.
    tool(d,3); d.drag((400,674),(650,724),release=False)
    assert len(d.data()['ink']) == 3
    d.shot('eraser-held.png'); d.key('Escape'); d.button(1,False)
    assert len(d.data()['ink']) == 3
    d.drag((400,674),(650,724))
    d.wait(lambda: len(d.data()['ink']) < 3, 'eraser did not commit')
    tool(d,0); chord(d,'z'); d.wait(lambda: len(d.data()['ink']) == 3,'Undo eraser failed')
    # The style panel changes saved, selected content.
    tool(d,0); d.key('Control',True); d.click(500,400); d.key('Control',False)
    d.click(1877,199)  # Red
    d.click(1835,292)  # Large
    d.drag((1882,256),(1810,256))
    d.click(1810,336)
    d.wait(lambda: annotations(d)[0]['style']['color'] == '#e03131', 'selected note color was not saved')
    assert annotations(d)[0]['style']['size'] == 36
    assert annotations(d)[0]['style']['opacity'] < 1
    d.shot('selected-note-style.png')
    chord(d,'c'); chord(d,'v')
    d.wait(lambda: len(annotations(d)) == 3, 'annotation Copy/Paste failed')
    chord(d,'z'); d.wait(lambda: len(annotations(d)) == 2, 'Undo annotation paste failed')
    # Right-click Edit opens the saved text through its real menu.
    d.click(500,400,3); d.shot('note-context.png')
    d.click(550,552)
    d.shot('note-edit-open.png')
    chord(d,'a'); d.text('Edited note'); d.click(1130,696)
    d.wait(lambda: annotations(d)[0]['name'] == 'Edited note', 'note edit did not save')
    # Image is chosen through the same native portal as the real user.
    image_path = d.work/'test-image.png'; Image.new('RGB',(600,300),(80,170,220)).save(image_path)
    tool(d,6); d.click(1000,350); time.sleep(1)
    d.shot('image-chooser.png'); chord(d,'l'); d.text(str(image_path)); d.key('Return'); time.sleep(1)
    d.x.XSetInputFocus(d.connection,d.window,1,0); d.flush()
    d.wait(lambda: len(annotations(d)) == 3, 'image was not imported')
    assert annotations(d)[2]['kind'] == 3
    d.shot('image-imported.png')
    # All saved content and the original image survive a normal daemon restart.
    saved = annotations(d); d.stop(); d.start(); assert annotations(d) == saved
    d.shot('tools-restarted.png')
    tool(d,3); d.drag((450,350),(500,400),release=False)
    assert len(annotations(d)) == 3
    d.shot('note-eraser-held.png'); d.key('Escape'); d.button(1,False)
    assert len(annotations(d)) == 3
    d.drag((450,350),(500,400)); d.wait(lambda: len(annotations(d)) == 2, 'note eraser did not commit')
    tool(d,0); chord(d,'z'); d.wait(lambda: len(annotations(d)) == 3,'Undo note eraser failed')
    # The test keyboard is ASCII only; enter mixed content via the saved profile,
    # then exercise the actual Note context menu, editor and Save path.
    d.stop(); fixture = d.data(); fixture['items'][0]['Annotation']['name'] = 'عمل Work\nملاحظة test'
    d.config.write_text(json.dumps(fixture)); d.start()
    d.click(500,400,3); d.click(550,552); d.shot('arabic-note-editor.png')
    d.click(1130,696); d.shot('arabic-note-saved.png')
    assert annotations(d)[0]['name'] == 'عمل Work\nملاحظة test'
    d.click(1808,56)
    d.shot('font-default.png')
    d.click(500,451); chord(d,'a'); d.text('24'); d.key('Return')
    d.key('Tab'); d.key('Tab'); d.key('Return')
    d.wait(lambda: d.data()['font_size'] == 24, 'global font size was not saved through Settings')
    d.shot('font-large.png')
    d.stop(); d.start(); assert d.data()['font_size'] == 24
    d.click(1808,56)
    d.x.XMoveResizeWindow(d.connection,d.window,0,0,1280,800); d.flush(); time.sleep(.5)
    d.shot('font-compact.png')
    print('PASS Canvas tools, styles, clipboard and global font setting:',d.work,flush=True)
finally:
    d.close()
    for service in reversed(services):
        service.terminate()
        try: service.wait(timeout=5)
        except subprocess.TimeoutExpired: service.kill()
    if bus_pid: os.kill(bus_pid, signal.SIGTERM)
