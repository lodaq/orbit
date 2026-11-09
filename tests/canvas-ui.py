"""Real launcher checks on a private Xvfb display and temporary Orbit profile.

Build with cargo build --release, then run python3 tests/canvas-ui.py.
Screenshots and the isolated profile remain under target/ui-check/canvas-*.
No input or IPC is sent to the user's compositor or resident Orbit daemon.
"""
import ctypes as C
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time

from PIL import ImageGrab

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/release/orbit'


class Desktop:
    def __init__(self, fixture=None):
        self.work = Path(tempfile.mkdtemp(prefix='canvas-', dir=REPO / 'target/ui-check'))
        self.config = self.work / 'config/orbit/config.json'
        self.config.parent.mkdir(parents=True)
        if fixture is not None:
            self.config.write_text(json.dumps(fixture))
        self.runtime = self.work / 'runtime'
        self.runtime.mkdir(mode=0o700)
        hypr = self.runtime / 'hypr/canvas-test'
        hypr.mkdir(parents=True)
        self.server = socket.socket(socket.AF_UNIX)
        self.server.bind(str(hypr / '.socket.sock'))
        self.server.listen()
        threading.Thread(target=self.serve, daemon=True).start()
        self.log = (self.work / 'daemon.log').open('a')
        read_fd, write_fd = os.pipe()
        self.xvfb = subprocess.Popen(
            ['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1920x1080x24', '-nolisten', 'tcp'],
            pass_fds=(write_fd,), stdout=self.log, stderr=self.log)
        os.close(write_fd)
        with os.fdopen(read_fd) as pipe:
            self.display = ':' + pipe.readline().strip()
        assert self.display != ':', 'Xvfb did not allocate a display'
        self.env = dict(os.environ, DISPLAY=self.display, WINIT_UNIX_BACKEND='x11',
                        XDG_CONFIG_HOME=str(self.work / 'config'),
                        XDG_RUNTIME_DIR=str(self.runtime), ORBIT_RUNTIME_DIRECTORY=str(self.runtime),
                        XDG_STATE_HOME=str(self.work / 'state'), XDG_CACHE_HOME=str(self.work / 'cache'),
                        HYPRLAND_INSTANCE_SIGNATURE='canvas-test',
                        DBUS_SESSION_BUS_ADDRESS='unix:path=' + str(self.runtime / 'no-secret-service'))
        self.env.pop('WAYLAND_DISPLAY', None)
        self.x = C.CDLL('libX11.so.6')
        self.xt = C.CDLL('libXtst.so.6')
        self.x.XOpenDisplay.argtypes = [C.c_char_p]
        self.x.XOpenDisplay.restype = C.c_void_p
        self.connection = self.x.XOpenDisplay(self.display.encode())
        assert self.connection
        self.x.XDefaultRootWindow.argtypes = [C.c_void_p]
        self.x.XDefaultRootWindow.restype = C.c_ulong
        self.root = self.x.XDefaultRootWindow(self.connection)
        self.x.XFlush.argtypes = [C.c_void_p]
        self.x.XQueryTree.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong),
                                      C.POINTER(C.c_ulong), C.POINTER(C.POINTER(C.c_ulong)),
                                      C.POINTER(C.c_uint)]
        self.x.XMoveResizeWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint]
        self.x.XFetchName.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(C.c_char_p)]
        self.x.XMapWindow.argtypes = [C.c_void_p, C.c_ulong]
        self.x.XSetInputFocus.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_ulong]
        self.x.XKeysymToKeycode.argtypes = [C.c_void_p, C.c_ulong]
        self.x.XKeysymToKeycode.restype = C.c_uint
        self.xt.XTestFakeKeyEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.xt.XTestFakeMotionEvent.argtypes = [C.c_void_p, C.c_int, C.c_int, C.c_int, C.c_ulong]
        self.xt.XTestFakeButtonEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.app = None

    def serve(self):
        while True:
            try:
                client, _ = self.server.accept()
                with client:
                    command = client.recv(8192).decode()
                    reply = 'ok'
                    if command == 'j/monitors':
                        reply = json.dumps([dict(name='screen', x=0, y=0, width=1920, height=1080,
                                                scale=1, transform=0, activeWorkspace=dict(id=1),
                                                specialWorkspace=dict(id=0))])
                    elif command == 'j/cursorpos':
                        reply = '{"x":0,"y":0}'
                    elif command == 'j/clients':
                        reply = '[]'
                    client.sendall(reply.encode())
            except OSError:
                return

    def wait(self, predicate, message, timeout=12):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if predicate():
                return
            if self.app is not None and self.app.poll() is not None:
                raise AssertionError('Orbit exited: ' + (self.work / 'daemon.log').read_text()[-3000:])
            time.sleep(.05)
        self.shot('failure.png')
        raise AssertionError(message + '; artifacts: ' + str(self.work))

    def start(self):
        self.app = subprocess.Popen([str(BINARY), 'daemon'], env=self.env, stdout=self.log, stderr=self.log)
        self.wait(lambda: (self.runtime / 'orbit/daemon.sock').exists(), 'daemon socket missing')
        self.toggle()
        time.sleep(1)
        parent, root, children, count = C.c_ulong(), C.c_ulong(), C.POINTER(C.c_ulong)(), C.c_uint()
        self.x.XQueryTree(self.connection, self.root, C.byref(root), C.byref(parent), C.byref(children), C.byref(count))
        assert count.value, 'launcher window missing'
        windows = []
        for i in range(count.value):
            name = C.c_char_p()
            if self.x.XFetchName(self.connection, children[i], C.byref(name)) and name.value == b'Orbit':
                windows.append(children[i])
        assert windows, 'Orbit window missing'
        self.window = windows[-1]
        self.x.XMoveResizeWindow(self.connection, windows[-1], 0, 0, 1920, 1080)
        self.x.XMapWindow(self.connection, windows[-1])
        self.x.XSetInputFocus(self.connection, windows[-1], 1, 0)
        self.flush()
        # Winit's initial fullscreen request can arrive after the first resize without a WM.
        def fullscreen_ready():
            self.x.XMoveResizeWindow(self.connection, windows[-1], 0, 0, 1920, 1080)
            self.flush()
            time.sleep(.1)
            return ImageGrab.grab(xdisplay=self.display).getpixel((1900, 500))[:3] != (0, 0, 0)
        self.wait(fullscreen_ready, 'launcher did not fill the private display')
        time.sleep(.5)

    def toggle(self):
        subprocess.run([str(BINARY), 'toggle'], env=self.env, check=True, timeout=15)

    def stop(self):
        if self.app is not None:
            self.app.terminate()
            self.app.wait(timeout=15)
            self.app = None

    def close(self):
        self.stop()
        self.xvfb.terminate()
        self.xvfb.wait(timeout=10)
        self.server.close()
        self.log.close()

    def flush(self):
        self.x.XFlush(self.connection)

    def move(self, x, y):
        self.xt.XTestFakeMotionEvent(self.connection, 0, round(x), round(y), 0)
        self.flush()
        time.sleep(.04)

    def button(self, button, pressed):
        self.xt.XTestFakeButtonEvent(self.connection, button, int(pressed), 0)
        self.flush()
        time.sleep(.04)

    def click(self, x, y, button=1):
        self.move(x, y)
        self.button(button, True)
        self.button(button, False)
        time.sleep(.25)

    def drag(self, start, end, release=True):
        self.move(*start)
        self.button(1, True)
        for step in range(1, 11):
            self.move(start[0] + (end[0] - start[0]) * step / 10,
                      start[1] + (end[1] - start[1]) * step / 10)
        if release:
            self.button(1, False)
        time.sleep(.3)

    def key(self, name, pressed=None):
        named = dict(Escape=0xff1b, Return=0xff0d, Tab=0xff09, BackSpace=0xff08,
                     Control=0xffe3, Shift=0xffe1, Left=0xff51, Right=0xff53,
                     Up=0xff52, Down=0xff54, PageUp=0xff55, PageDown=0xff56)
        symbol = named[name] if name in named else ord(name)
        code = self.x.XKeysymToKeycode(self.connection, symbol)
        for state in ([True, False] if pressed is None else [pressed]):
            self.xt.XTestFakeKeyEvent(self.connection, code, int(state), 0)
        self.flush()
        time.sleep(.03)

    def text(self, value):
        assert value.isascii(), 'test keyboard helper accepts ASCII; use fixtures for Arabic'
        for letter in value:
            if letter.isupper() or letter in ':_':
                self.key('Shift', True)
                self.key({':': ';', '_': '-'}.get(letter, letter.lower()))
                self.key('Shift', False)
            else:
                self.key(letter)
        time.sleep(.2)

    def wheel(self, x, y, steps):
        self.move(x, y)
        for _ in range(abs(steps)):
            self.button(4 if steps > 0 else 5, True)
            self.button(4 if steps > 0 else 5, False)
        time.sleep(.4)

    def zoom(self, x, y, steps):
        self.key('Control', True)
        try:
            self.wheel(x, y, steps)
        finally:
            self.key('Control', False)

    def shot(self, name):
        path = self.work / name
        ImageGrab.grab(xdisplay=self.display).save(path)
        return path

    def data(self):
        return json.loads(self.config.read_text())

    def items(self):
        return [dict(value, item_type=kind) for item in self.data()['items']
                for kind, value in item.items()]

    def empty_point(self):
        data = self.data()
        view = data['canvas']['view']
        for y in range(220, 850, 90):
            for x in range(160, 1760, 90):
                wx, wy = (x - view['x']) / view['zoom'], (y - view['y']) / view['zoom']
                if not any(r['col'] <= wx < r['col'] + r['width']
                           and r['row'] <= wy < r['row'] + r['height']
                           for r in data['positions'].values()):
                    return x, y
        raise AssertionError('No visible empty point in test layout')

    def open_add(self, kind='App'):
        x, y = self.empty_point()
        self.click(x, y, 3)
        self.click(x + 70, y + 28)
        if kind != 'App':
            self.click(744 + ['App', 'Website', 'File', 'Directory', 'Folder', 'Widgets'].index(kind) * 86, 292)
        time.sleep(.3)

    def item_point(self, item_id, corner=False):
        data = self.data()
        r, v = data['positions'][item_id], data['canvas']['view']
        gap = data['grid_gap'] / 2
        x = r['col'] + (r['width'] - gap - 6 if corner else gap + 30)
        y = r['row'] + (r['height'] - gap - 6 if corner else gap + 20)
        return v['x'] + x * v['zoom'], v['y'] + y * v['zoom']

    def replace_text(self, value):
        self.key('Control', True)
        self.key('a')
        self.key('Control', False)
        self.text(value)


def main():
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--interactive', action='store_true', help='open a private test desktop for scripted inspection')
    args = parser.parse_args()
    (REPO / 'target/ui-check').mkdir(parents=True, exist_ok=True)
    desktop = Desktop()
    try:
        desktop.start()
        print('Isolated desktop:', desktop.work, flush=True)
        if args.interactive:
            import code
            code.interact(local=dict(d=desktop, REPO=REPO))
        else:
            verify(desktop)
    finally:
        desktop.close()


def verify(d):
    d.shot('entry.png')
    assert d.data()['version'] == 5
    assert 'pages' not in d.data(), 'live pages survived migration'
    before = d.data()['canvas']['view']
    d.click(40, 516)  # Hand tool pans; Select now draws a selection rectangle.
    d.drag((800, 450), (650, 550))
    d.wait(lambda: d.data()['canvas']['view'] != before, 'empty drag did not pan')
    saved = d.data()['canvas']['view']
    d.zoom(1000, 500, -2)
    d.wait(lambda: d.data()['canvas']['view']['zoom'] < saved['zoom'], 'wheel did not zoom')
    saved = d.data()['canvas']['view']
    d.stop()
    d.start()
    assert d.data()['canvas']['view'] == saved, 'view did not survive restart'
    d.click(900, 500, 3)
    d.shot('context.png')
    d.click(950, 520)
    d.shot('add-item.png')
    d.key('Escape')
    assert d.data()['canvas']['view'] == saved, 'cancelled sheet changed the view'
    print('PASS isolated launch, pan, zoom, restart and right-click Add item', flush=True)
    verify_items(d)
    verify_widgets(d)



def verify_items(d):
    d.open_add()
    d.click(850, 449); d.text('CanvasCheck')
    d.click(850, 550); d.text('true')
    d.click(1167, 869)
    d.wait(lambda: len(d.items()) == 1, 'custom-command App was not saved')
    app_id = d.items()[0]['id']
    for kind, path in [('File', d.work / 'canvas-file.txt'), ('Directory', d.work)]:
        if kind == 'File':
            path.write_text('Canvas test')
        before = len(d.items())
        d.open_add(kind)
        d.click(850, 385); d.text(str(path))
        d.click(1167, 620)
        d.wait(lambda: len(d.items()) == before + 1, kind + ' was not saved')
        assert any(item.get('path') == str(path) for item in d.items())
        d.shot('saved-' + kind + '.png')
    d.open_add('Folder')
    d.click(850, 385); d.text('CanvasFolder')
    d.click(1167, 566)
    d.wait(lambda: len(d.items()) == 4, 'Folder was not saved')
    folder_id = next(item['id'] for item in d.items() if item['item_type'] == 'Folder')
    d.shot('saved-folder.png')
    # A successful free move changes only the dragged rectangle.
    before = d.data()['positions']
    d.drag(d.item_point(app_id), (1250, 500))
    d.wait(lambda: d.data()['positions'][app_id] != before[app_id], 'App did not move freely')
    assert all(rect == d.data()['positions'][key] for key, rect in before.items() if key != app_id)
    before = d.data()['positions']
    d.drag(d.item_point(app_id), (1500, 700), release=False)
    d.key('Escape'); d.button(1, False)
    assert d.data()['positions'] == before, 'cancelled drag was saved'
    # Pin and Unpin use the visible item and dock context menus.
    x, y = d.item_point(app_id)
    d.click(x, y, 3); d.click(x + 70, y + 68)
    d.wait(lambda: app_id in d.data()['pinned'], 'Pin did not transfer ownership to Dock')
    assert app_id not in d.data()['positions']
    d.shot('pinned.png')
    d.click(960, 1020, 3); d.click(1030, 1048)
    d.wait(lambda: app_id not in d.data()['pinned'], 'Unpin did not restore Canvas placement')
    # Folder creation still requires the completed shortcut overlap hold.
    file_id = next(item['id'] for item in d.items() if item['item_type'] == 'File')
    d.drag(d.item_point(file_id), d.item_point(app_id), release=False)
    time.sleep(.6); d.button(1, False)
    d.wait(lambda: any(len(item.get('apps', [])) == 2 for item in d.items()), 'held overlap did not create a Folder')
    merged = next(item for item in d.items() if len(item.get('apps', [])) == 2)
    assert {app['id'] for app in merged['apps']} == {file_id, app_id}
    assert any(app['launch'].get('File') == str(d.work / 'canvas-file.txt')
               for app in merged['apps'] if isinstance(app['launch'], dict))
    d.click(*d.item_point(merged['id']))
    d.shot('opened-folder.png'); d.key('Escape')
    # Search keeps its normal result presentation and clears with Escape.
    d.click(900, 58); d.text('CanvasCheck')
    d.shot('search-results.png'); d.key('Escape')
    d.click(1758, 56)
    d.wait(lambda: d.data()['layout_locked'], 'Lock layout did not save')
    before = d.data()['positions']
    d.drag(d.item_point(folder_id), (1500, 500))
    assert d.data()['positions'] == before, 'locked layout accepted a move'
    view = d.data()['canvas']['view']
    d.zoom(*d.empty_point(), -1)
    d.wait(lambda: d.data()['canvas']['view'] != view, 'Lock layout prevented zoom')
    d.click(1758, 56)
    d.wait(lambda: not d.data()['layout_locked'], 'Unlock layout did not save')
    before_count = len(d.items())
    d.open_add('Website')
    d.click(865, 537)
    d.click(850, 510); d.text('W')
    d.click(1167, 706)
    d.click(850, 385); d.text('CanvasWebsite')
    d.click(850, 473); d.text('https://example.com')
    d.click(1167, 754)
    d.wait(lambda: len(d.items()) == before_count + 1, 'Website was not saved')
    assert any(item.get('url') == 'https://example.com' for item in d.items())
    d.shot('saved-website.png')
    print('PASS all launcher item types, free move, cancellation, Dock, folder hold, search and lock', flush=True)


def verify_widgets(d):
    variants = list(range(7, 19)) + [20]
    for index, variant in enumerate(variants):
        before = {item['id'] for item in d.items()}
        d.open_add('Widgets')
        d.zoom(952, 700, 16)
        if index // 2:
            d.zoom(952, 700, -(index // 2) * 2)
        # The final fully visible Auto button belongs to the requested gallery row.
        x = 730 if index % 2 == 0 else 992
        screenshot = ImageGrab.grab(xdisplay=d.display)
        blue = [y for y in range(370, 795)
                if (lambda r, g, b: b > 150 and b > r * 1.3 and 50 < r < 120)(*screenshot.getpixel((x, y))[:3])]
        assert blue, f'Widget {variant} gallery button missing'
        y = blue[-1] - 18
        place = index == 2
        d.click(x + (156 if place else 40), y)
        if place:
            d.move(1200, 500)
            d.shot('placement-preview.png')
            d.click(1200, 500)
        d.wait(lambda: len(d.items()) == len(before) + 1, f'Widget {variant} was not added')
        added = next(item for item in d.items() if item['id'] not in before)
        assert added['variant'] == variant, (variant, added)
        time.sleep(.5)
        d.shot(f'widget-{variant}.png')
        if index == 2:
            before_positions = d.data()['positions']
            corner = d.item_point(added['id'], corner=True)
            d.drag(corner, (corner[0] + 40, corner[1] + 20))
            d.wait(lambda: d.data()['positions'][added['id']] != before_positions[added['id']], 'widget resize failed')
            assert all(rect == d.data()['positions'][key] for key, rect in before_positions.items() if key != added['id'])
            d.shot('resized-widget.png')
            target = next(item for item in d.items() if item.get('variant') == 7)
            before_positions = d.data()['positions']
            d.drag(d.item_point(added['id']), d.item_point(target['id']), release=False)
            d.shot('invalid-widget-move.png'); d.button(1, False)
            time.sleep(.4)
            assert d.data()['positions'] == before_positions, 'occupied widget drop changed saved positions'
            # Widget body scrolling must not zoom the Canvas.
            x, y = d.item_point(added['id'])
            before_view = d.data()['canvas']
            d.wheel(x + 30, y + 80, -1)
            assert d.data()['canvas'] == before_view, 'widget body wheel zoomed the Canvas'
            print('PASS widget resize, occupied-drop rollback and content scrolling', flush=True)
        print(f'PASS right-click -> Widgets -> {added["name"]} -> {"Place" if place else "Auto"}', flush=True)
    positions = list(d.data()['positions'].values())
    for index, a in enumerate(positions):
        for b in positions[index + 1:]:
            assert not (a['col'] < b['col'] + b['width'] and b['col'] < a['col'] + a['width']
                        and a['row'] < b['row'] + b['height'] and b['row'] < a['row'] + a['height'])
    point = d.empty_point()
    d.click(*point, 3)
    d.click(point[0] + 70, point[1] + 68)
    time.sleep(.5)
    d.shot('all-widgets.png')
    data = d.data()
    assert data['canvas']['view']['zoom'] <= 2
    d.stop(); d.start()
    assert d.data()['positions'] == data['positions']
    assert d.data()['canvas'] == data['canvas']
    print('PASS nonoverlapping widget positions and saved view survive restart', flush=True)
    verify_navigation_and_failed_migration(d.data())


def verify_navigation_and_failed_migration(template):
    fixture = json.loads(json.dumps(template))
    fixture['items'] = [dict(App=dict(id=item_id, name=item_id, icon='Letter', launch=dict(Command='true')))
                        for item_id in ['pinned', 'near', 'far']]
    fixture['pinned'] = ['pinned']
    fixture['positions'] = {item_id: dict(col=col, row=200, width=144, height=144)
                            for item_id, col in [('near', 100), ('far', 100000)]}
    fixture['canvas'] = dict(view=dict(x=0, y=0, zoom=1))
    other = Desktop(fixture)
    try:
        other.start()
        other.click(800, 800)
        other.key('Right'); other.key('Right')
        other.wait(lambda: other.data()['canvas']['view']['x'] < -90000,
                   'keyboard focus did not reveal the distant item')
        other.shot('keyboard-reveal.png')
        point = other.empty_point()
        other.click(*point, 3); other.click(point[0] + 70, point[1] + 68)
        other.wait(lambda: other.data()['canvas']['view']['zoom'] < .25, 'Show all items did not fit distant items')
        other.shot('fit-below-quarter.png')
        before = other.data()['canvas']['view']
        other.zoom(900, 800, 1)
        other.wait(lambda: other.data()['canvas']['view']['zoom'] > before['zoom'], 'fitted view did not zoom in')
        after = other.data()['canvas']['view']
        assert after['zoom'] < before['zoom'] * 1.5, 'zoom jumped from a fitted view'
        for coordinate, pointer in [('x', 900), ('y', 800)]:
            assert abs((pointer - before[coordinate]) / before['zoom']
                       - (pointer - after[coordinate]) / after['zoom']) < .1
        print('PASS keyboard reveal, fit below 25% and smooth pointer-centered zoom', flush=True)
    finally:
        other.close()
    fixture.pop('items')
    fixture.update(version=1, columns=8, rows=6, active_page=0, pinned=[],
                   pages=[dict(items=[dict(File=dict(id='overflow', name='saved', path='/tmp/saved', icon='Letter'))])],
                   positions=dict(overflow=dict(col=2147483647, row=0, width=1, height=1)))
    other = Desktop(fixture)
    original = other.config.read_bytes()
    try:
        other.start()
        other.click(1758, 56)
        other.wait(lambda: 'restart Orbit before saving changes' in (other.work / 'daemon.log').read_text(),
                   'failed migration did not block saving the empty fallback')
        assert other.config.read_bytes() == original
        backups = list(other.config.parent.glob('config-before-canvas-*.json'))
        assert backups and backups[0].read_bytes() == original
        other.shot('migration-error-preserves-original.png')
        print('PASS failed migration preserves original bytes and rejects later saves', flush=True)
    finally:
        other.close()


if __name__ == '__main__':
    main()
