"""Opt-in real Hyprland gesture checks. Uses isolated data and restarts Orbit.
Run with ORBIT_INPUT_SOCKET pointing to a running ydotoold socket.
The compositor's ydotool absolute coordinate scale is supplied explicitly.
Set ORBIT_TEST_SHORTCUTS=1 to run the same gestures with Website shortcuts.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/release/orbit'
WORK = Path(tempfile.mkdtemp(prefix='orbit-gestures-'))
RUNTIME = WORK / 'runtime'
RUNTIME.mkdir()
ENV = dict(os.environ, XDG_CONFIG_HOME=str(WORK),
           ORBIT_RUNTIME_DIRECTORY=str(RUNTIME),
           YDOTOOL_SOCKET=os.environ['ORBIT_INPUT_SOCKET'])
SCALE = float(os.environ.get('ORBIT_POINTER_SCALE', '1'))
CONFIG = WORK / 'orbit/config.json'
CONFIG.parent.mkdir()
process = None


def command(*args, **kwargs):
    return subprocess.run(args, env=ENV, check=True, stdout=subprocess.DEVNULL, **kwargs)


def move(x, y):
    command('ydotool', 'mousemove', '-a', '-x', str(round(x / SCALE)), '-y', str(round(y / SCALE)))
    time.sleep(.08)


def button(value):
    command('ydotool', 'click', value)
    time.sleep(.08)


def drag(source, target, dwell=.65, release=True):
    move(*source); button('0x40'); move(*target); time.sleep(dwell)
    if release:
        button('0x80'); time.sleep(.25)


def click(x, y):
    move(x, y); button('0xc0'); time.sleep(.25)


def read():
    return json.loads(CONFIG.read_text())


def ids(data):
    return [item for page in data['pages'] for item in page['items']]


def folders():
    return [item['Folder'] for item in ids(read()) if 'Folder' in item]


def stop():
    global process
    if process:
        process.terminate(); process.wait(timeout=5); process = None


def start(reset=True):
    global process
    stop()
    if reset:
        apps = [{'App': {'id': name, 'name': 'Test ' + name, 'icon': 'Letter',
                         'launch': {'Command': '/usr/bin/true'}}} for name in 'ABCD']
        if os.environ.get('ORBIT_TEST_SHORTCUTS') == '1':
            apps = [{'Website': {'id': name, 'name': 'Test ' + name, 'icon': 'Letter',
                                 'url': 'https://example.com/' + name}} for name in 'ABCD']
        CONFIG.write_text(json.dumps({'version': 3, 'pages': [{'id': 'test-page', 'items': apps}], 'pinned': [], 'positions': {name: {'col': n * 144, 'row': 0, 'width': 144, 'height': 144} for n, name in enumerate('ABCD')}}))
    process = subprocess.Popen([BINARY, 'daemon'], env=ENV, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    time.sleep(.35)
    command(str(BINARY), 'toggle'); time.sleep(1)


def screenshot(name):
    command('grim', str(WORK / (name + '.png')))


def check(name, test):
    start(); test(); screenshot(name); print('PASS', name, flush=True)


# 1920×1080, margin 10, cell 128: first-row icons are centered at x=82+144*n, y=148.
A, B, C, D = [(82 + 144*n, 148) for n in range(4)]


def center():
    drag(A, B, release=False); screenshot('center-preview'); button('0x80'); time.sleep(.3)
    assert [app['id'] for app in folders()[0]['apps']] == ['B', 'A']
    saved = read(); start(False); assert read() == saved


def edge():
    drag(A, (190,148), release=False); screenshot('edge-preview'); button('0x80'); time.sleep(.3)
    data = read(); assert not folders()
    assert data['positions']['A']['col'] == 0 and data['positions']['B']['col'] == 144


def right_edge():
    drag(D, (274,148), release=False); screenshot('right-edge-preview')
    button('0x80'); time.sleep(.3)
    positions = read()['positions']
    assert not folders() and positions['D']['col'] == 432
    assert positions['B']['col'] == 144 and positions['C']['col'] == 288


def staggered():
    stop()
    data = read()
    for name, (col, row) in zip('ABCD', [(0, 0), (300, 120), (470, 200), (700, 20)]):
        data['positions'][name].update(col=col, row=row)
    CONFIG.write_text(json.dumps(data)); start(False)
    initial = read()
    drag(A, (362,318), release=False)
    screenshot('staggered-preview')
    assert read() == initial
    move(364,318); move(362,318); time.sleep(.4)
    button('0x80'); time.sleep(.3)
    positions = read()['positions']
    assert positions == initial['positions']
    saved = read(); start(False); assert read() == saved


def staggered_right_and_cancel():
    stop()
    data = read()
    for name, (col, row) in zip('ABCD', [(0, 0), (300, 120), (470, 200), (700, 20)]):
        data['positions'][name].update(col=col, row=row)
    CONFIG.write_text(json.dumps(data)); start(False)
    initial = read()
    # Occupied edges keep every item fixed, both on cancel and release.
    drag(A, (402,318), release=False); screenshot('staggered-right-preview')
    assert read() == initial
    command('wtype','-k','Escape'); button('0x80'); time.sleep(.2)
    assert read() == initial
    drag(A, (402,318))
    positions = read()['positions']
    assert positions == initial['positions']


def quick():
    initial = read(); drag(A,B,dwell=.02)
    assert read() == initial


def switch_intent():
    drag(A,B,release=False); move(190,148); time.sleep(.4); button('0x80'); time.sleep(.3)
    assert not folders() and read()['positions']['A']['col'] == 0


def empty():
    drag(B,(802,148))
    positions = read()['positions']; assert positions['B']['col'] == 720
    assert positions['A']['col'] == 0 and positions['C']['col'] == 288
    saved = read(); start(False); assert read() == saved


def cancel():
    initial = read(); drag(A,(802,148),release=False)
    command('wtype','-k','Escape'); button('0x80'); time.sleep(.2)
    assert read() == initial


def outside():
    initial = read()
    drag(A, (190,148), release=False)
    move(900,50); button('0x80'); time.sleep(.3)
    assert read() == initial


def dock():
    drag(A,(960,1000)); data = read(); assert data['pinned'] == ['A']
    drag((960,1000),(802,148)); data = read(); assert data['pinned'] == []
    assert data['positions']['A']['col'] == 752


def cross_page():
    drag(A,(1900,400),dwell=.7,release=False); move(802,148); time.sleep(.15); button('0x80'); time.sleep(.3)
    data = read(); assert len(data['pages']) == 2
    assert next(iter(data['pages'][1]['items'][0].values()))['id'] == 'A'
    saved = read(); start(False); assert read() == saved


def temporary_page():
    initial = read(); drag(A,(1900,400),dwell=.7,release=False)
    command('wtype','-k','Escape'); button('0x80'); time.sleep(.2)
    assert read() == initial


def folder_round_trip():
    drag(A,B); drag(C,B); click(*B); screenshot('folder-open')
    drag((1024,546),(802,300))
    assert len(folders()) == 1 and len(folders()[0]['apps']) == 2
    assert read()['positions']['C']['row'] == 140
    if os.environ.get('ORBIT_TEST_SHORTCUTS') == '1':
        assert any(item.get('Website', {}).get('url') == 'https://example.com/C' for item in ids(read()))


def dissolution():
    drag(A,B); click(*B); drag((752,546),(802,300))
    assert not folders() and read()['positions']['A']['col'] == 144


def folder_edge():
    drag(A, B)
    folder_id = folders()[0]['id']
    drag(C, (190,148), release=False); screenshot('folder-edge-preview')
    button('0x80'); time.sleep(.3)
    assert len(folders()[0]['apps']) == 2
    assert read()['positions'][folder_id]['col'] == 144
    assert read()['positions']['C']['col'] == 288


def edge_to_center():
    drag(A,(190,148),release=False); move(*B); time.sleep(.65); button('0x80'); time.sleep(.3)
    assert [app['id'] for app in folders()[0]['apps']] == ['B','A']


try:
    for name, test in [('center',center),('edge',edge),('right-edge',right_edge),('staggered',staggered),('staggered-right-cancel',staggered_right_and_cancel),('quick-center',quick),
                       ('center-to-edge',switch_intent),('edge-to-center',edge_to_center),('folder-edge',folder_edge),('folder-round-trip',folder_round_trip),('dissolution',dissolution),('empty',empty),('cancel',cancel),('outside',outside),
                       ('dock',dock),('cross-page',cross_page),('temporary-page',temporary_page)]:
        check(name,test)
finally:
    stop()
    print('Screenshots and isolated saved data:', WORK, flush=True)
