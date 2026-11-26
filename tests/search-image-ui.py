"""Search anything image paste, chip UI, engine switching, and text fallback on a private display."""
import importlib
import json
import subprocess
import time
from PIL import Image

Desktop = importlib.import_module('canvas-ui').Desktop


def chord(d, key):
    d.key('Control', True)
    d.key(key)
    d.key('Control', False)


def ocr(d, name, box):
    path = d.shot(name + '.png')
    crop = path.with_name(name + '-crop.png')
    w = (box[2] - box[0]) * 3
    h = (box[3] - box[1]) * 3
    Image.open(path).crop(box).resize((w, h)).save(crop)
    return subprocess.check_output(
        ['tesseract', str(crop), 'stdout', '--psm', '6'],
        stderr=subprocess.DEVNULL,
    ).decode()


def wait_ocr(d, name, box, expected):
    def check():
        return expected.lower() in ocr(d, name, box).lower()

    d.wait(check, f'expected {expected!r} in {name}')


def chip_teal_count(d, name):
    path = d.shot(name + '.png')
    crop = Image.open(path).crop((725, 44, 753, 72)).convert('RGB')
    colors = crop.getcolors(28 * 28) or []
    return sum(
        count
        for count, color in colors
        if abs(color[0] - 60) < 25 and abs(color[1] - 200) < 25 and abs(color[2] - 180) < 25
    )


def set_clipboard_image(d, picture):
    proc = subprocess.Popen(
        [
            '/usr/bin/python3',
            '-c',
            '''
import gi, sys
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk, GdkPixbuf
clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
clipboard.set_image(GdkPixbuf.Pixbuf.new_from_file(sys.argv[1]))
print('ready', flush=True)
Gtk.main()
''',
            str(picture),
        ],
        env=d.env,
        stdout=subprocess.PIPE,
        stderr=d.log,
        text=True,
    )
    assert proc.stdout.readline().strip() == 'ready'
    return proc


def set_clipboard_text(d, text):
    proc = subprocess.Popen(
        [
            '/usr/bin/python3',
            '-c',
            '''
import gi, sys
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk
clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
clipboard.set_text(sys.argv[1], -1)
print('ready', flush=True)
Gtk.main()
''',
            text,
        ],
        env=d.env,
        stdout=subprocess.PIPE,
        stderr=d.log,
        text=True,
    )
    assert proc.stdout.readline().strip() == 'ready'
    return proc


ENGINE_BOX = (535, 34, 675, 84)
SEARCH_BOX = (720, 36, 1170, 82)
SEARCH_AFTER_CHIP_BOX = (780, 36, 1170, 82)
RESULTS_BOX = (650, 100, 1270, 320)

d = Desktop()
owner = None
try:
    picture = d.work / 'search-clip.png'
    Image.new('RGBA', (160, 160), (60, 200, 180, 255)).save(picture)
    owner = set_clipboard_image(d, picture)

    d.start()
    d.stop()
    fixture = d.data()
    fixture['default_search_engine'] = 'perplexity'
    fixture['smooth_cursor'] = False
    d.config.write_text(json.dumps(fixture))
    d.start()

    wait_ocr(d, 'initial-engine', ENGINE_BOX, 'Perplexity')

    # Paste image in Search anything: chip appears and engine switches to Google.
    chord(d, 'v')
    d.wait(lambda: chip_teal_count(d, 'pasted-chip') > 400, 'image chip did not appear')
    wait_ocr(d, 'google-engine', ENGINE_BOX, 'Google')

    # Typing /script text while image is attached does not open Scripts or local results.
    d.text('/orbit-test')
    wait_ocr(d, 'image-query', SEARCH_AFTER_CHIP_BOX, 'orbit-test')
    assert 'Scripts' not in ocr(d, 'no-scripts', RESULTS_BOX)

    # Clicking the chip's × button removes the image and lets /orbit-test enter Scripts mode;
    # pressing Escape then clears /orbit-test and shows the restored Perplexity selector.
    d.click(765, 58)
    d.wait(lambda: chip_teal_count(d, 'after-x') < 50, 'image chip did not clear on ×')
    wait_ocr(d, 'scripts-after-x', RESULTS_BOX, 'Scripts')
    d.key('Escape')
    wait_ocr(d, 'reset-after-x', ENGINE_BOX, 'Perplexity')

    # Paste image, type plain text, click ×: chip clears and Perplexity resets immediately while text stays.
    d.click(860, 58)
    time.sleep(0.2)
    chord(d, 'v')
    d.wait(lambda: chip_teal_count(d, 'pasted-plain') > 400, 'plain paste failed')
    wait_ocr(d, 'google-plain', ENGINE_BOX, 'Google')
    d.text('plain')
    wait_ocr(d, 'plain-query', SEARCH_AFTER_CHIP_BOX, 'plain')
    d.click(765, 58)
    d.wait(lambda: chip_teal_count(d, 'after-x-plain') < 50, 'image chip did not clear on × with plain text')
    wait_ocr(d, 'reset-after-x-plain', ENGINE_BOX, 'Perplexity')
    wait_ocr(d, 'kept-plain-text', SEARCH_BOX, 'plain')

    # Clear text with Escape, paste again, then Backspace on empty text removes chip and resets engine.
    d.key('Escape')
    time.sleep(0.2)
    d.click(860, 58)
    time.sleep(0.2)
    chord(d, 'v')
    d.wait(lambda: chip_teal_count(d, 'pasted-again') > 400, 'second image paste failed')
    wait_ocr(d, 'google-again', ENGINE_BOX, 'Google')
    d.key('BackSpace')
    d.wait(lambda: chip_teal_count(d, 'after-backspace') < 50, 'Backspace did not remove image chip')
    wait_ocr(d, 'reset-after-backspace', ENGINE_BOX, 'Perplexity')

    # Paste image, type text, and press Escape: clears both image and text and resets engine.
    chord(d, 'v')
    d.wait(lambda: chip_teal_count(d, 'pasted-esc') > 400, 'third image paste failed')
    d.text('red square')
    wait_ocr(d, 'before-esc', SEARCH_AFTER_CHIP_BOX, 'red square')
    d.key('Escape')
    d.wait(lambda: chip_teal_count(d, 'after-esc') < 50, 'Escape did not clear image chip')
    wait_ocr(d, 'reset-after-esc', ENGINE_BOX, 'Perplexity')
    wait_ocr(d, 'cleared-search', SEARCH_BOX, 'Search anything')

    # Text-only clipboard still pastes text natively when no image is present.
    owner.terminate()
    owner.wait(timeout=5)
    owner = set_clipboard_text(d, 'hello orbit')
    d.click(860, 58)
    time.sleep(0.2)
    chord(d, 'v')
    wait_ocr(d, 'pasted-text', SEARCH_BOX, 'hello orbit')
    assert chip_teal_count(d, 'no-chip-on-text') < 50
    wait_ocr(d, 'still-perplexity', ENGINE_BOX, 'Perplexity')

    # Full Enter submit with fake xdg-open: serves handoff HTML, resets search & engine, and hides.
    fake_bin = d.work / 'bin'
    fake_bin.mkdir()
    xdg = fake_bin / 'xdg-open'
    xdg.write_text(f'''#!/usr/bin/env python3
import sys, urllib.request
url = sys.argv[1]
outer = urllib.request.urlopen(url, timeout=5).read().decode()
assert 'src="/post"' in outer, outer
post = urllib.request.urlopen(url.rstrip('/') + '/post', timeout=5).read().decode()
open({str(d.work / 'opened-post.html')!r}, 'w').write(post)
open({str(d.work / 'opened-url.txt')!r}, 'w').write(url)
''')
    xdg.chmod(0o755)
    d.stop()
    d.env['PATH'] = f"{fake_bin}:{d.env['PATH']}"
    d.start()
    owner.terminate()
    owner.wait(timeout=5)
    owner = set_clipboard_image(d, picture)
    chord(d, 'v')
    d.wait(lambda: chip_teal_count(d, 'pasted-before-enter') > 400, 'image paste before Enter failed')
    wait_ocr(d, 'google-before-enter', ENGINE_BOX, 'Google')
    d.text('what is this')
    wait_ocr(d, 'text-before-enter', SEARCH_AFTER_CHIP_BOX, 'what is this')
    d.key('Return')
    d.wait(lambda: (d.work / 'opened-post.html').exists(), 'Enter did not open Lens handoff URL')
    post_html = (d.work / 'opened-post.html').read_text()
    assert 'https://lens.google.com/v3/upload?hl=en&q=what%20is%20this' in post_html
    assert 'encoded_image' in post_html
    d.toggle()
    time.sleep(0.5)
    d.x.XSetInputFocus(d.connection, d.window, 1, 0)
    d.flush()
    wait_ocr(d, 'reset-after-submit', ENGINE_BOX, 'Perplexity')
    wait_ocr(d, 'cleared-after-submit', SEARCH_BOX, 'Search anything')

    print('PASS search-bar image paste, Google switch, reset, text fallback, and Enter handoff:', d.work)
finally:
    d.close()
    if owner:
        owner.terminate()
        owner.wait(timeout=5)
