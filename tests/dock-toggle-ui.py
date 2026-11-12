"""Dock-revealed special workspaces close on the first toggle."""
import importlib
import json
import time

Desktop = importlib.import_module('canvas-ui').Desktop


class DockDesktop(Desktop):
    workspace = '1'
    special_open = False

    def serve(self):
        while True:
            try:
                client, _ = self.server.accept()
                with client:
                    command = client.recv(8192).decode()
                    reply = 'ok'
                    if command == 'j/monitors':
                        reply = json.dumps([dict(name='screen', x=0, y=0, width=1920,
                            height=1080, scale=1, transform=0,
                            activeWorkspace=dict(id=1, name='1'),
                            specialWorkspace=dict(id=-99 if self.special_open else 0,
                                name='special:orbit' if self.special_open else ''))])
                    elif command == 'j/cursorpos':
                        reply = '{"x":0,"y":0}'
                    elif command == 'j/clients':
                        reply = json.dumps([dict(pid=self.app.pid, fullscreen=2,
                            workspace=dict(name=self.workspace))]) if self.app else '[]'
                    elif command == 'dispatch togglespecialworkspace orbit':
                        reply = 'error: legacy dispatcher syntax is unsupported'
                    elif command == 'dispatch hl.dsp.workspace.toggle_special("orbit")':
                        self.special_open = not self.special_open
                    elif 'window.move(' in command:
                        self.workspace = 'special:orbit' if 'special:orbit' in command else '1'
                    client.sendall(reply.encode())
            except OSError:
                return


d = DockDesktop()
try:
    d.start()
    d.text('kept draft')
    d.toggle()
    d.x.XSetInputFocus(d.connection, d.root, 1, 0)
    d.flush()
    time.sleep(.4)
    assert d.workspace == 'special:orbit'
    # Dock activation reveals the existing parking workspace, without Orbit IPC.
    d.special_open = True
    d.x.XSetInputFocus(d.connection, d.window, 1, 0)
    d.flush()
    time.sleep(.4)
    d.shot('dock-open.png')
    d.toggle()
    time.sleep(.4)
    assert not d.special_open, 'First toggle left the dock-revealed special workspace open'
    for _ in range(3):
        d.toggle()
        time.sleep(.3)
        assert d.workspace == '1', 'Toggle did not reopen Orbit'
        d.toggle()
        time.sleep(.3)
        assert d.workspace == 'special:orbit' and not d.special_open, 'Toggle did not hide Orbit'
    print('PASS dock workspace closes with one toggle and repeated toggles:', d.work)
finally:
    d.close()
