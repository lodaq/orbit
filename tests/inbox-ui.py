"""Inbox widget check on a private display/profile. Build release first."""
import importlib
import os
import sqlite3
import subprocess
import time

c = importlib.import_module('canvas-ui')

d = c.Desktop()


def field(x, y, value):
    d.click(x, y)
    d.key('Control', True)
    d.key('a')
    d.key('Control', False)
    d.text(value)


def inbox():
    path = d.work / 'config/orbit/widgets.db'
    if not path.exists():
        return []
    connection = sqlite3.connect(path)
    rows = connection.execute('SELECT id,query FROM inbox_tasks ORDER BY rowid').fetchall()
    connection.close()
    return rows


def widget():
    """The screen position of the Inbox widget body and its first Add button."""
    data = d.data()
    for item in data['items']:
        for kind, value in item.items():
            if kind == 'Widget' and value['variant'] == 21:
                rect = data['positions'][value['id']]
                view = data['canvas']['view']
                gap = data['grid_gap'] / 2
                size = data['item_size']
                left = view['x'] + (rect['col'] + gap + 16) * view['zoom']
                top = view['y'] + (rect['row'] + gap + 48) * view['zoom']
                body = (rect['width'] - data['grid_gap'] - 32) * view['zoom']
                return left, top, left + body - 68, top + 24
    raise AssertionError('the Inbox widget is not on the Canvas')


try:
    d.start()
    # The Widget Gallery is the real entry point for the Inbox card.
    d.open_add('Widgets')
    time.sleep(.5)
    d.wheel(960, 600, -40)
    d.shot('gallery.png')
    d.click(769, 778)
    d.wait(lambda: any(item['Widget']['variant'] == 21
                       for item in d.data()['items'] if 'Widget' in item),
           'the Inbox widget was not added')
    d.shot('widget-added.png')

    # A task that TickTick cannot accept waits in the Inbox widget.
    subprocess.run([str(c.BINARY), 'ticktick'], env=d.env, check=True, timeout=15)
    time.sleep(.8)
    field(900, 56, 'buy milk')
    d.key('Return')
    d.wait(lambda: len(inbox()) == 1, 'the failed task was not kept')
    assert inbox()[0][1] == 'buy milk', inbox()
    d.shot('task-kept.png')

    d.key('Escape')
    time.sleep(.5)

    # The retry keeps the task when TickTick still refuses it.
    x, y, button_x, button_y = widget()
    d.click(button_x, button_y)
    time.sleep(1)
    d.shot('retry.png')
    assert len(inbox()) == 1, 'a failed retry removed the task'
    d.click(button_x + 48, button_y)
    time.sleep(.4)
    d.shot('delete-confirm.png')
    d.click(960, 591)
    d.wait(lambda: len(inbox()) == 0, 'delete did not remove the task')
    d.shot('empty.png')
    connection = sqlite3.connect(d.work / 'config/orbit/widgets.db')
    connection.executemany('INSERT INTO inbox_tasks(id,query) VALUES(?,?)', [
        ('arabic', 'شراء الحليب !high #المنزل'),
        ('mixed', 'مراجعة Orbit مع الفريق #work'),
    ])
    connection.commit()
    connection.close()
    subprocess.run([str(c.BINARY), 'toggle'], env=d.env, check=True, timeout=15)
    subprocess.run([str(c.BINARY), 'toggle'], env=d.env, check=True, timeout=15)
    time.sleep(1)
    d.shot('multilingual.png')
    print('PASS', d.work, flush=True)
finally:
    d.close()
