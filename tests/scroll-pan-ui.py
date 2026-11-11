"""Wheel pans keep the Canvas while widgets move under the pointer.

Run after cargo build --release. Uses a private display and test profile.
"""
import importlib
import json

Desktop = importlib.import_module('canvas-ui').Desktop

d = Desktop()
try:
    d.start()
    d.stop()
    fixture = d.data()
    fixture.update(items=[dict(Widget=dict(id='widget', name='Random Quote', text='Keep going.',
                                          variant=9, ticktick_project_id='',
                                          min_number=1, max_number=100))],
                   positions={'widget': dict(col=650, row=600, width=400, height=340)},
                   frames=[], ink=[], canvas=dict(view=dict(x=0, y=0, zoom=1)),
                   canvas_speed=1)
    d.config.write_text(json.dumps(fixture))
    d.start()
    positions = d.data()['positions']
    d.shot('pan-before.png')

    # Xvfb reports both button edges, so four notches produce eight wheel events.
    # The first events start outside; the same sequence crosses the Widget.
    d.move(800, 450)
    for _ in range(4):
        d.button(5, True)
        d.button(5, False)
    d.shot('pan-wheel-end.png')
    d.wait(lambda: d.data()['canvas']['view']['y'] == -480,
           'vertical pan stopped when the Widget reached the pointer')
    d.shot('pan-over-widget.png')

    # After idle, a new scroll inside Widget content must leave the Canvas still.
    view = d.data()['canvas']['view']
    d.wheel(800, 300, -3)
    assert d.data()['canvas']['view'] == view, 'Widget scroll moved the Canvas'

    d.move(1200, 300)
    d.key('Shift', True)
    for _ in range(4):
        d.button(4, True)
        d.button(4, False)
    d.key('Shift', False)
    d.wait(lambda: d.data()['canvas']['view']['x'] == 480,
           'horizontal pan stopped when the Widget reached the pointer')
    d.shot('pan-horizontal.png')
    assert d.data()['positions'] == positions, 'panning moved the saved Widget'
    saved = d.data()['canvas']['view']
    d.stop()
    d.start()
    assert d.data()['canvas']['view'] == saved, 'pan did not survive restart'
    d.shot('pan-restarted.png')
    print('PASS continuous wheel panning, Widget scroll and restart:', d.work, flush=True)
finally:
    d.close()
