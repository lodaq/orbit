"""Script launcher check on a private display/profile. Build release first."""
import importlib
import json
import os
from pathlib import Path
import subprocess
import time

c = importlib.import_module('canvas-ui')
class ScriptDesktop(c.Desktop):
    commands = []

    def serve(self):
        while True:
            try:
                client, _ = self.server.accept()
                with client:
                    command = client.recv(8192).decode()
                    self.commands.append(command)
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

    def hidden_count(self):
        return sum('special:orbit' in command for command in self.commands)

d = ScriptDesktop()
services = []
bus_pid = None

def chord(key):
    d.key('Control', True)
    d.key(key)
    d.key('Control', False)

def field(x, y, value):
    d.click(x, y)
    chord('a')
    d.text(value)

try:
    # All portal services and their chooser share the private X display and bus.
    bus = subprocess.check_output(['dbus-daemon', '--session', '--fork', '--print-address=1', '--print-pid=1'], env=d.env, text=True).splitlines()
    d.env['DBUS_SESSION_BUS_ADDRESS'] = bus[0]
    bus_pid = int(bus[1])
    d.env['XDG_CURRENT_DESKTOP'] = 'GNOME'
    d.env['XDG_DATA_HOME'] = str(d.work / 'data')
    (d.work / 'data').mkdir()
    portal_config = d.work / 'config/xdg-desktop-portal/portals.conf'
    portal_config.parent.mkdir(parents=True)
    portal_config.write_text('[preferred]\ndefault=gtk\n')
    for program in ['xdg-desktop-portal-gtk', 'xdg-desktop-portal']:
        services.append(subprocess.Popen(['/usr/lib/' + program], env=d.env, stdout=d.log, stderr=d.log))
    time.sleep(1)
    script = d.work / 'deploy'
    script.write_text('#!/bin/sh\nprintf "%s\\n" "$#" "$PWD" "$1" > launched\n')
    script.chmod(0o700)
    d.start()
    d.click(1808, 56)
    d.shot('settings-entry.png')
    d.click(120, 404)
    d.click(680, 232)
    d.shot('scripts-empty.png')
    print('ARTIFACTS', d.work, flush=True)
    field(500, 392, 'deploy')
    field(500, 484, '/does-not-exist')
    d.click(1765, 552)
    time.sleep(.3)
    assert not d.data().get('scripts'), 'invalid path was saved'
    d.shot('invalid-path.png')
    # Error adds a line above the action row.
    chooser_hidden = d.hidden_count()
    d.click(380, 580)
    time.sleep(1)
    d.shot('native-chooser.png')
    chord('l')
    d.text(str(script))
    d.key('Return')
    time.sleep(1)
    d.shot('chosen-path.png')
    assert d.hidden_count() == chooser_hidden, 'chooser focus hid the launcher'
    # Xvfb has no window manager to return X input focus after the chooser closes.
    d.x.XSetInputFocus(d.connection, d.window, 1, 0)
    d.flush()
    d.click(1765, 552)
    d.wait(lambda: len(d.data().get('scripts', [])) == 1, 'script was not saved')
    assert d.data()['scripts'][0]['path'] == str(script)
    script_id = d.data()['scripts'][0]['id']
    d.shot('script-saved.png')
    d.click(1720, 336)
    field(500, 458, 'release')
    d.click(1765, 618)
    d.wait(lambda: d.data()['scripts'][0]['name'] == 'release', 'edit was not saved')
    assert d.data()['scripts'][0]['id'] == script_id
    alpha = d.work / 'alpha'
    alpha.write_text('#!/bin/sh\nprintf alpha > alpha-launched\n')
    alpha.chmod(0o700)
    d.click(1630, 618)
    field(500, 458, 'alpha')
    field(500, 550, str(alpha))
    d.click(1765, 618)
    d.wait(lambda: len(d.data()['scripts']) == 2, 'second script missing')
    d.shot('scripts-edited.png')
    d.click(110, 886)
    field(900, 56, '/no-match')
    d.key('Return')
    time.sleep(.3)
    assert not (d.work / 'launched').exists()
    d.shot('no-match.png')
    field(900, 56, '/RElease  argument text')
    d.key('Tab')
    d.shot('filtered.png')
    hidden = d.hidden_count()
    d.key('Return')
    output = d.work / 'launched'
    d.wait(output.exists, 'script did not run')
    assert output.read_text() == '1\n' + str(d.work) + '\n argument text\n', repr(output.read_text())
    d.shot('launched.png')
    d.wait(lambda: d.hidden_count() > hidden, 'successful launch did not hide Orbit')
    d.toggle()
    time.sleep(.4)
    output.unlink()
    script.chmod(0o600)
    field(900, 56, '/release')
    hidden = d.hidden_count()
    d.key('Return')
    time.sleep(.4)
    assert not output.exists() and d.hidden_count() == hidden, 'failed launch hid Orbit'
    d.shot('launch-error.png')
    script.chmod(0o700)
    field(900, 56, '/')
    for key in ['Up', 'Down', 'Up', 'Down']:
        d.key(key)
    d.shot('keyboard-selection.png')
    d.key('Return')
    time.sleep(.3)
    assert not output.exists(), 'first Enter ran instead of completing'
    d.shot('enter-completed.png')
    d.key('Return')
    d.wait(output.exists, 'keyboard selection did not launch release')
    assert output.read_text() == '1\n' + str(d.work) + '\n\n'
    assert not (d.work / 'alpha-launched').exists()
    d.toggle()
    time.sleep(.4)
    output.unlink()
    field(900, 56, '/')
    d.key('Down')
    d.key('Tab')
    d.key('Tab')
    time.sleep(.2)
    assert not output.exists(), 'Tab launched a script'
    d.shot('tab-completed.png')
    d.text('selected argument')
    d.shot('selected-arguments.png')
    d.key('Return')
    d.wait(output.exists, 'typing arguments changed the selected script')
    assert output.read_text() == '1\n' + str(d.work) + '\nselected argument\n'
    d.toggle()
    time.sleep(.4)
    field(900, 56, '/')
    d.click(900, 170)
    d.wait(lambda: (d.work / 'alpha-launched').exists(), 'click did not launch alpha')
    d.toggle()
    time.sleep(.4)
    # Escape exits the picker without launching or hiding.
    field(900, 56, '/')
    hidden = d.hidden_count()
    d.key('Escape')
    time.sleep(.3)
    assert d.hidden_count() == hidden
    d.shot('escaped.png')
    d.stop()
    d.start()
    assert len(d.data()['scripts']) == 2 and d.data()['scripts'][0]['id'] == script_id
    d.click(1808, 56)
    d.click(120, 404)
    d.click(680, 232)
    d.shot('restarted.png')
    d.click(1790, 336)
    d.wait(lambda: len(d.data()['scripts']) == 1, 'delete did not remove alpha')
    assert alpha.exists() and script.exists(), 'delete removed a script file'
    d.shot('deleted.png')
    # Exercise a list longer than six rows, duplicate names, and Arabic labels.
    d.stop()
    config = d.data()
    last = d.work / 'last'
    last.write_text('#!/bin/sh\nprintf last > last-launched\n')
    last.chmod(0o700)
    config['scripts'] += [dict(id='overflow-' + str(i), name='entry' + str(i), path=str(alpha)) for i in range(7)]
    config['scripts'] += [dict(id='duplicate', name='release', path=str(alpha)), dict(id='arabic', name='نشر', path=str(last))]
    d.config.write_text(json.dumps(config))
    d.start()
    (d.work / 'alpha-launched').unlink()
    output.unlink()
    field(900, 56, '/')
    for _ in range(8):
        d.key('Down')
    d.key('Tab')
    time.sleep(.3)
    d.shot('duplicate-completed.png')
    d.key('Return')
    d.wait(lambda: (d.work / 'alpha-launched').exists(), 'completion changed the duplicate script selection')
    assert not output.exists()
    d.toggle()
    time.sleep(.4)
    field(900, 56, '/')
    for _ in range(20):
        d.key('Down')
    time.sleep(.2)
    d.shot('scroll-arabic.png')
    d.key('Return')
    time.sleep(.2)
    d.key('Return')
    d.wait(lambda: (d.work / 'last-launched').exists(), 'last keyboard row did not run')
    d.toggle()
    time.sleep(.4)
    d.click(1808, 56)
    d.click(120, 404)
    d.click(680, 232)
    d.x.XMoveResizeWindow(d.connection, d.window, 0, 0, 1280, 800)
    d.flush()
    time.sleep(.5)
    d.shot('compact-list.png')
    d.wheel(1000, 600, -30)
    d.shot('compact-editor.png')
    print('PASS', d.work, flush=True)
finally:
    d.close()
    for service in reversed(services):
        service.terminate()
        service.wait(timeout=10)
    if bus_pid:
        os.kill(bus_pid, 15)
