"""Exercise Robot management and chat on a private Xvfb display and profile."""
import ctypes as C
import http.server
import json
from pathlib import Path
import runpy
import sqlite3
import threading
import time

Desktop = runpy.run_path(str(Path(__file__).with_name('canvas-ui.py')))['Desktop']


class Provider(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers.get('Content-Length', 0))))
        messages = request.get('messages', [])
        last = messages[-1] if messages else {}
        if self.path == '/api/show':
            response = dict(capabilities=['tools'])
        elif last.get('role') == 'user' and last.get('content', '').startswith('Run this exact shell command'):
            script = last['content'].split('\n\n', 1)[1]
            response = dict(done=True, message=dict(role='assistant', content='', tool_calls=[dict(function=dict(name='run', arguments=dict(program='sh', arguments=['-c', script])))]))
        else:
            response = dict(done=True, message=dict(role='assistant', content='Ready. تم تنفيذ الطلب التجريبي.'))
        body = json.dumps(response).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


def count(db, table, kind=None):
    if kind is None:
        return db.execute(f'SELECT COUNT(*) FROM {table}').fetchone()[0]
    return db.execute(f"SELECT COUNT(*) FROM {table} WHERE json_extract(data,'$.kind')=?", (kind,)).fetchone()[0]


def resize(d, width, height):
    root, parent, children, size = C.c_ulong(), C.c_ulong(), C.POINTER(C.c_ulong)(), C.c_uint()
    d.x.XQueryTree(d.connection, d.root, C.byref(root), C.byref(parent), C.byref(children), C.byref(size))
    for index in range(size.value):
        name = C.c_char_p()
        if d.x.XFetchName(d.connection, children[index], C.byref(name)) and name.value == b'Orbit':
            d.x.XMoveResizeWindow(d.connection, children[index], 0, 0, width, height)
    d.flush()
    time.sleep(.3)


def verify(d, db, skill):
    def wait_count(table, expected, kind=None):
        d.wait(lambda: count(db, table, kind) == expected, f'{table}/{kind}: expected {expected}')

    def nav(index):
        d.click(100, 150 + index * 48)

    def robot_point():
        data = d.data()
        rect, view = data['positions']['tile'], data['canvas']['view']
        return (round(view['x'] + (rect['col'] + rect['width'] / 2) * view['zoom']),
                round(view['y'] + (rect['row'] + rect['height'] / 2) * view['zoom']))

    d.click(120, 214)
    d.click(340, 786)
    d.wait(lambda: count(db, 'entries', 'Reply') == 1, 'prompt reply missing')
    d.shot('prompt-result.png')
    d.click(510, 786)
    d.wait(lambda: db.execute("SELECT COUNT(*) FROM entries WHERE json_extract(data,'$.status')='AwaitingApproval'").fetchone()[0] > 0, 'shell approval missing')
    assert not db.execute("SELECT COUNT(*) FROM entries WHERE json_extract(data,'$.output') LIKE '%orbit-shell-check%' AND json_extract(data,'$.status')='Completed'").fetchone()[0]
    d.shot('shell-approval.png')
    d.click(426, 734)
    d.wait(lambda: db.execute("SELECT COUNT(*) FROM entries WHERE json_extract(data,'$.output') LIKE '%orbit-shell-check%' AND json_extract(data,'$.status')='Completed'").fetchone()[0] > 0, 'approved shell output missing')
    d.shot('shell-result.png')
    print('PASS prompt reply, exact shell approval and output', flush=True)

    d.key('Escape')
    x, y = robot_point()
    d.click(x, y, 3)
    d.shot('robot-menu.png')
    d.click(x + 55, y + 28)
    wait_count('profiles', 2)
    d.wait(lambda: len(d.items()) == 2, 'duplicate Canvas item missing')
    positions = d.data()['positions']
    x, y = robot_point()
    d.click(x, y, 3)
    d.click(x + 55, y + 72)
    d.wait(lambda: count(db, 'entries', 'Reply') == 3, 'context action did not run again')
    d.shot('repeated-action-result.png')
    d.key('Escape')
    d.click(1806, 56)
    d.click(130, 260)
    time.sleep(.3)
    d.shot('management.png')

    nav(1)
    d.click(420, 470); d.text('Extra skill')
    d.click(420, 546); d.text(str(skill))
    d.click(307, 598)
    wait_count('catalog', 2, 'skill')
    d.click(1750, 394); d.click(1750, 394)
    wait_count('catalog', 1, 'skill')
    extra = skill.parent.with_name('new-skill') / 'SKILL.md'
    extra.parent.mkdir(); extra.write_text('New installed skill')
    d.click(360, 185)
    d.shot('skills-refreshed.png')

    nav(2)
    d.click(360, 185)
    wait_count('accounts', 1)
    d.shot('providers-fetched.png')
    d.click(400, 475); d.text('Local test')
    d.click(330, 603)
    wait_count('accounts', 2)
    d.click(1788, 237); d.click(1788, 237)
    wait_count('accounts', 1)

    nav(6)
    d.click(420, 325); d.text('Extra project')
    d.click(420, 401); d.text(str(d.work))
    d.click(307, 453)
    wait_count('catalog', 2, 'project')
    d.click(1750, 185); d.click(1750, 185)
    wait_count('catalog', 1, 'project')

    nav(7)
    d.click(420, 313); d.text('Reviewer')
    d.click(420, 420); d.text('Check changes carefully')
    d.click(306, 581)
    wait_count('roles', 1)
    d.click(340, 249); d.click(340, 249)
    wait_count('roles', 0)

    nav(3)
    d.click(420, 389); d.text('Review change')
    d.click(420, 541); d.text('Review this project')
    d.click(307, 617)
    wait_count('catalog', 2, 'prompt')
    d.click(1670, 249); d.click(1670, 249)
    wait_count('catalog', 1, 'prompt')
    d.shot('actions.png')
    print('PASS Skills refresh and Skill/provider/project/Role/Action add-remove', flush=True)

    nav(4)
    d.click(420, 371); d.text('fixture-mcp')
    d.click(420, 447); d.text('/usr/bin/true')
    d.click(319, 952)
    wait_count('catalog', 1, 'mcp')
    d.shot('mcp-added.png')

    nav(0)
    d.click(308, 185)
    d.click(420, 389); d.text('Extra robot')
    d.click(420, 465); d.text(str(d.work))
    d.click(1820, 1035)
    wait_count('profiles', 3)
    d.click(1788, 365); d.click(1788, 365)
    wait_count('profiles', 2)
    d.click(420, 243)
    d.click(420, 389)
    d.key('Control', True); d.key('a'); d.key('Control', False)
    d.text('Edited helper')
    d.click(1820, 1035)
    d.wait(lambda: d.data()['items'][0]['Robot']['name'] == 'Edited helper', 'edited Robot name did not reach Canvas')
    assert d.data()['positions'] == positions
    d.shot('robot-edited.png')

    d.key('Escape'); d.key('Escape')
    d.stop(); d.start()
    assert len(d.items()) == 2
    wait_count('catalog', 1, 'mcp')
    d.click(*robot_point())
    d.shot('chat-after-restart.png')
    d.key('Escape')
    d.click(1806, 56); d.click(130, 260)
    nav(4)
    d.click(330, 220)
    d.click(330, 307)
    wait_count('catalog', 0, 'mcp')
    d.key('Escape'); d.key('Escape')
    d.click(*robot_point())
    resize(d, 1280, 800)
    d.shot('chat-compact.png')
    d.key('Escape')
    d.click(1166, 56); d.click(130, 260)
    nav(7)
    d.key('Tab'); d.key('Tab')
    d.shot('roles-compact.png')
    nav(4)
    d.shot('mcps-compact.png')
    d.wheel(1100, 650, -8)
    d.shot('mcps-compact-bottom.png')
    nav(5)
    d.shot('credentials-compact.png')
    print('PASS MCP persistence/removal, Robot create/edit/remove and compact chat/management', flush=True)


def main():
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Provider)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    fixture = dict(version=5, items=[dict(Robot=dict(id='tile', name='Build helper', profile='helper'))],
                   pinned=[], positions=dict(tile=dict(col=400, row=240, width=144, height=144)),
                   canvas=dict(view=dict(x=0, y=0, zoom=1)))
    d = Desktop(fixture)
    d.env['HOME'] = str(d.work / 'home')
    skill = d.work / 'home/.agents/skills/research/SKILL.md'
    skill.parent.mkdir(parents=True)
    skill.write_text('---\nname: Research\ndescription: Test skill\n---\nCite sources.\n')
    t3_settings = d.work / 'home/.t3/userdata/settings.json'
    t3_settings.parent.mkdir(parents=True)
    t3_settings.write_text(json.dumps(dict(providerInstances=dict(fixture=dict(driver='codex', displayName='T3 fixture', enabled=True)))))
    model = dict(provider='Ollama', id='fixture', ollama_address=f'http://127.0.0.1:{server.server_port}', account=None)
    profile = dict(id='helper', name='Build helper · مساعد', project=str(d.work), model=model,
                   role_id=None, skills=[], mcp_servers=[])
    db_path = d.config.with_name('robots.db')
    db = sqlite3.connect(db_path)
    db.executescript('''
        CREATE TABLE roles(id TEXT PRIMARY KEY NOT NULL, data TEXT NOT NULL);
        CREATE TABLE profiles(id TEXT PRIMARY KEY NOT NULL, role_id TEXT REFERENCES roles(id), data TEXT NOT NULL);
        CREATE TABLE conversations(id TEXT PRIMARY KEY NOT NULL, robot_id TEXT NOT NULL REFERENCES profiles(id),
            title TEXT NOT NULL, context TEXT NOT NULL, created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL, rating INTEGER);
        CREATE TABLE entries(conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            sequence INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(conversation_id,sequence));
        CREATE TABLE accounts(id TEXT PRIMARY KEY NOT NULL, data TEXT NOT NULL);
        CREATE TABLE catalog(id TEXT PRIMARY KEY NOT NULL, data TEXT NOT NULL);
        PRAGMA user_version=4;
    ''')
    db.execute('INSERT INTO profiles VALUES(?,NULL,?)', ('helper', json.dumps(profile)))
    context = dict(project=str(d.work), role=None, model=model)
    db.execute('INSERT INTO conversations VALUES(?,?,?,?,?,?,NULL)',
               ('chat', 'helper', 'Check project', json.dumps(context), 1, 1))
    db.execute('INSERT INTO entries VALUES(?,?,?)', ('chat', 0, json.dumps(dict(kind='Command', text='Check project'))))
    for entry in [dict(id='check', kind='prompt', name='Check project', content='Check project'),
                  dict(id='shell', kind='shell', name='Shell check', content='printf orbit-shell-check'),
                  dict(id='project', kind='project', name='Fixture project', content=str(d.work)),
                  dict(id='custom-skill', kind='skill', name='Custom research', content=str(skill))]:
        db.execute('INSERT INTO catalog VALUES(?,?)', (entry['id'], json.dumps(entry)))
    db.commit()
    try:
        d.start()
        d.shot('robots-canvas.png')
        d.click(470, 310)
        time.sleep(.5)
        d.shot('robots-chat.png')
        print(d.work, flush=True)
        verify(d, db, skill)
    finally:
        db.close()
        d.close()
        server.shutdown()


if __name__ == '__main__':
    main()
