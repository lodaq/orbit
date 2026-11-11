"""Real widget controls and transfers on private display/profile/network namespaces.

Requires sudo -n, nftables, iproute2, iperf3, Xvfb and tesseract. Build release first.
The user's network rules, profile, display and input are never changed.
"""
import importlib
import json
import os
from pathlib import Path
import shlex
import subprocess as s
import sys
import time

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
c = importlib.import_module('canvas-ui')
d = c.Desktop()
ns = 'orbit-limit-' + str(os.getpid())
peer = ns + '-peer'
leaf = ns + '-leaf'
server = None
namespaces = []

def run(*args, **kwargs):
    return s.run(args, check=True, capture_output=True, text=True, timeout=30, **kwargs).stdout

def net(name, *args, **kwargs):
    return run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'netns', 'exec', name, *args, **kwargs)

def words():
    path = d.shot('current.png')
    tsv = run('tesseract', str(path), 'stdout', 'tsv')
    rows = []
    for line in tsv.splitlines()[1:]:
        p = line.split('\t')
        if len(p) >= 12 and p[0] == '5' and p[11].strip():
            rows.append({'text': p[11].strip(), 'left': int(p[6]), 'top': int(p[7]),
                         'width': int(p[8]), 'height': int(p[9])})
    return rows

def text():
    return ' '.join(r['text'] for r in words())

def click_word(label, last=False):
    found = [r for r in words() if r['text'] == label]
    assert found, (label, text(), d.work)
    r = found[-1] if last else found[0]
    d.click(int(r['left']) + int(r['width']) / 2, int(r['top']) + int(r['height']) / 2)

def click_limit():
    item = next(i for i in d.items() if i['item_type'] == 'Widget' and i['name'] == 'Internet Usage')
    rect = d.data()['positions'][item['id']]
    view = d.data()['canvas']['view']
    gap = d.data()['grid_gap'] / 2
    d.click(view['x'] + rect['col'] + rect['width'] - gap - 76,
            view['y'] + rect['row'] + gap + 84)

def rates():
    entries = json.loads(net(ns, '/usr/bin/nft', '-j', 'list', 'tables'))['nftables']
    if not any(e.get('table', {}).get('name') == 'orbit_speed_limits' for e in entries):
        return {}
    entries = json.loads(net(ns, '/usr/bin/nft', '-j', 'list', 'table', 'inet', 'orbit_speed_limits'))['nftables']
    return {e['rule']['chain']: next(x['limit']['rate'] for x in e['rule']['expr'] if 'limit' in x)
            for e in entries if 'rule' in e}

def set_limits(down, up):
    rows = sorted((r for r in words()
                   if r['text'].startswith('(') and 'p' in r['text'] and 700 < r['left'] < 1300),
                  key=lambda r: r['top'])[:2]
    assert len(rows) == 2, text()
    for row, value in zip(rows, (down, up)):
        d.click(900, int(row['top']) + 40)
        d.replace_text(value)
    image = Image.open(d.shot('apply.png')).convert('RGB')
    blue = [(x, y) for y in range(760, image.height) for x in range(900, image.width)
            if (lambda p: p[2] > 150 and p[2] > p[0] * 1.3 and 50 < p[0] < 140)(image.getpixel((x, y)))]
    assert blue, 'Apply limits button missing'
    d.click((min(x for x, _ in blue) + max(x for x, _ in blue)) / 2,
            (min(y for _, y in blue) + max(y for _, y in blue)) / 2)
    time.sleep(.4)

def transfer(reverse=False, ipv6=False, cap=None, source=None, parallel=1):
    result = json.loads(net(source or ns, '/usr/bin/iperf3', '-c', 'fd42:203::2' if ipv6 else '10.203.0.2',
                           '-t', '6', '-O', '1', '-P', str(parallel), '-J', *(['-R'] if reverse else [])))
    assert 'error' not in result, result
    speed = result['end']['sum_received']['bits_per_second'] / 1e6
    label = ('IPv6' if ipv6 else 'IPv4') + (' download' if reverse else ' upload')
    print(f'{label}: {speed:.3f} Mbps (cap {cap})', flush=True)
    if cap:
        assert cap * .35 < speed < cap * 1.3, (label, speed, cap)
    else:
        assert speed > 20, (label, speed)
    return speed

try:
    # Create the widget through its actual gallery entry before entering a private network.
    d.start()
    d.open_add('Widgets')
    d.wheel(1100, 640, -36)
    gallery = d.shot('gallery.png')
    assert 'Internet Usage' in text(), text()
    image = Image.open(gallery).convert('RGB')
    blue = [(x, y) for y in range(730, image.height) for x in range(image.width)
            if (lambda p: p[2] > 150 and p[2] > p[0] * 1.3 and 50 < p[0] < 140)(image.getpixel((x, y)))]
    assert blue, 'Internet Usage Auto button missing'
    d.click((min(x for x, _ in blue) + max(x for x, _ in blue)) / 2,
            (min(y for _, y in blue) + max(y for _, y in blue)) / 2)
    d.wait(lambda: any(i['item_type'] == 'Widget' and i['name'] == 'Internet Usage' for i in d.items()), 'widget not added')
    d.stop()
    for name in (ns, peer, leaf):
        run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'netns', 'add', name)
        namespaces.append(name)
    run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'link', 'add', 'orb-limit0', 'type', 'veth', 'peer', 'name', 'orb-limit1')
    for name, dev, address, v6 in [(ns, 'orb-limit0', '10.203.0.1/24', 'fd42:203::1/64'),
                                  (peer, 'orb-limit1', '10.203.0.2/24', 'fd42:203::2/64')]:
        run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'link', 'set', dev, 'netns', name)
        for args in [('addr', 'add', address, 'dev', dev), ('-6', 'addr', 'add', v6, 'dev', dev, 'nodad'),
                     ('link', 'set', dev, 'up'), ('link', 'set', 'lo', 'up')]:
            net(name, '/usr/bin/ip', *args)
    run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'link', 'add', 'orb-leaf0', 'type', 'veth', 'peer', 'name', 'orb-leaf1')
    for name, dev, address in [(ns, 'orb-leaf0', '10.204.0.1/24'), (leaf, 'orb-leaf1', '10.204.0.2/24')]:
        run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'link', 'set', dev, 'netns', name)
        net(name, '/usr/bin/ip', 'addr', 'add', address, 'dev', dev)
        net(name, '/usr/bin/ip', 'link', 'set', dev, 'up')
    net(ns, '/usr/bin/sysctl', '-qw', 'net.ipv4.ip_forward=1')
    net(leaf, '/usr/bin/ip', 'route', 'add', 'default', 'via', '10.204.0.1')
    net(peer, '/usr/bin/ip', 'route', 'add', '10.204.0.0/24', 'via', '10.203.0.1')
    net(ns, '/usr/bin/nft', '-f', '-', input='add table inet keep_me\nadd chain inet keep_me input { type filter hook input priority 10; policy accept; }\n')
    sentinel = net(ns, '/usr/bin/nft', '-j', 'list', 'table', 'inet', 'keep_me')
    stub = d.work / 'bin'
    stub.mkdir()
    failure = d.work / 'deny-nft'
    (stub / 'sudo').write_text('#!/bin/sh\nif [ "$3" = "-f" ] && [ -e ' + shlex.quote(str(failure)) + ' ]; then echo "simulated administrator access denied" >&2; exit 1; fi\nexec /usr/bin/sudo "$@"\n')
    (stub / 'sudo').chmod(0o755)
    d.env['PATH'] = str(stub) + ':' + os.environ['PATH']
    wrapper = d.work / 'orbit-in-network'
    preserved = ['DISPLAY', 'WINIT_UNIX_BACKEND', 'XDG_CONFIG_HOME', 'XDG_RUNTIME_DIR', 'ORBIT_RUNTIME_DIRECTORY',
                 'XDG_STATE_HOME', 'XDG_CACHE_HOME', 'HYPRLAND_INSTANCE_SIGNATURE', 'DBUS_SESSION_BUS_ADDRESS', 'PATH']
    assignments = ' '.join(f'{key}="${key}"' for key in preserved)
    wrapper.write_text('#!/bin/sh\nexec /usr/bin/sudo -n /usr/bin/ip netns exec ' + shlex.quote(ns) +
                       ' /usr/bin/env ' + assignments + ' ' + shlex.quote(str(c.BINARY)) + ' "$@"\n')
    wrapper.chmod(0o755)
    c.BINARY = wrapper
    server = s.Popen(['/usr/bin/sudo', '-n', '/usr/bin/ip', 'netns', 'exec', peer, '/usr/bin/iperf3', '-s'],
                     stdout=d.log, stderr=d.log)
    d.start()
    click_limit()
    d.wait(lambda: 'Unlimited' in text(), 'active state not visible')
    d.shot('unlimited.png')
    set_limits('2000', '4000')
    d.wait(lambda: rates() == dict(download=250000, upload=500000), 'caps not applied')
    assert '2000 kbps' in text() and '4000 kbps' in text(), text()
    d.shot('applied.png')
    print('PASS gallery → widget → Limit → Apply; active rates visible', flush=True)
    transfer(cap=4)
    transfer(reverse=True, cap=2)
    transfer(ipv6=True, cap=4)
    transfer(reverse=True, ipv6=True, cap=2)
    transfer(cap=4, parallel=3)
    transfer(reverse=True, cap=2, parallel=3)
    transfer(source=leaf)
    transfer(source=leaf, reverse=True)
    before = rates()
    set_limits('-1', '4000')
    assert rates() == before and 'Enter a whole number' in text(), text()
    d.shot('invalid.png')
    set_limits('6000', '3000')
    d.wait(lambda: rates() == dict(download=750000, upload=375000), 'editing caps failed')
    failure.touch()
    set_limits('9000', '8000')
    d.wait(lambda: 'Administrator access' in text(), 'permission failure not shown')
    assert rates() == dict(download=750000, upload=375000)
    d.shot('permission-error.png')
    failure.unlink()
    d.key('Escape')
    click_limit()
    assert '6000' in text() and '3000' in text() and 'kbps' in text(), text()
    d.stop()
    d.start()
    click_limit()
    assert '6000' in text() and '3000' in text() and 'kbps' in text(), text()
    d.shot('restart-active.png')
    set_limits('0', '3000')
    d.wait(lambda: rates() == dict(upload=375000), 'independent download removal failed')
    transfer(reverse=True)
    transfer(cap=3)
    set_limits('2000', '0')
    d.wait(lambda: rates() == dict(download=250000), 'independent upload removal failed')
    transfer()
    transfer(reverse=True, cap=2)
    click_word('Remove')
    d.wait(lambda: rates() == {}, 'Remove limits failed')
    assert text().count('Unlimited') >= 2, text()
    d.shot('removed.png')
    transfer()
    transfer(reverse=True)
    assert net(ns, '/usr/bin/nft', '-j', 'list', 'table', 'inet', 'keep_me') == sentinel
    # Refuse a table-name collision without touching it.
    net(ns, '/usr/bin/nft', '-f', '-', input='delete table inet orbit_speed_limits\nadd table inet orbit_speed_limits { comment "another tool"; }\n')
    d.key('Escape')
    click_limit()
    assert 'another tool' in text(), text()
    d.shot('table-conflict.png')
    print('PASS IPv4/IPv6 transfers, asymmetric/independent caps, validation, failure atomicity, restart, removal and unrelated firewall preservation', flush=True)
    print('Artifacts:', d.work, flush=True)
finally:
    d.close()
    if server:
        server.terminate()
        server.wait(timeout=10)
    for name in namespaces:
        for pid in run('/usr/bin/sudo', '-n', '/usr/bin/ip', 'netns', 'pids', name).split():
            s.run(['/usr/bin/sudo', '-n', 'kill', pid], capture_output=True)
        s.run(['/usr/bin/sudo', '-n', '/usr/bin/ip', 'netns', 'del', name], capture_output=True)
