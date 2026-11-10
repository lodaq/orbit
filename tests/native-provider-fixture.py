#!/usr/bin/env python3
"""Offline stdio fixture for Orbit's native provider protocol tests."""
import json
import os
from pathlib import Path
import sys
import time

work = Path(os.environ['ORBIT_NATIVE_FIXTURE'])
work.joinpath('started').write_text(str(os.getpid()))
protocol = 'codex' if 'app-server' in sys.argv else 'pi' if 'rpc' in sys.argv else 'acp'
pending = None


def send(value):
    print(json.dumps(value), flush=True)


def reply(request, result):
    send(dict(jsonrpc='2.0', id=request['id'], result=result))


for line in sys.stdin:
    request = json.loads(line)
    method = request.get('method', request.get('type'))
    if method == 'initialize':
        reply(request, dict(protocolVersion=1))
    elif method == 'thread/start':
        assert request['params']['approvalPolicy'] == 'untrusted'
        assert request['params']['sandbox'] == 'read-only'
        reply(request, dict(thread=dict(id='thread')))
    elif method == 'session/new':
        options = [dict(id='model', category='model')] if work.joinpath('config-model').exists() else []
        reply(request, dict(sessionId='session', configOptions=options))
    elif method == 'session/set_config_option':
        work.joinpath('selected-model').write_text(request['params']['value'])
        reply(request, {})
    elif method in ('session/set_model', 'set_model'):
        if protocol == 'pi':
            send(dict(type='response', id=request['id'], success=True))
        else:
            reply(request, {})
    elif method in ('session/prompt', 'turn/start', 'prompt'):
        if work.joinpath('hang').exists():
            time.sleep(30)
        if protocol == 'pi':
            if work.joinpath('pi-error').exists():
                send(dict(type='message_end', message=dict(role='assistant', stopReason='error', errorMessage='seeded-secret')))
                send(dict(type='agent_end', messages=[], willRetry=True))
                continue
            send(dict(type='agent_end', messages=[dict(role='assistant', content=[dict(type='text', text='مرحبا Orbit')])]))
            continue
        pending = request
        if protocol == 'codex':
            reply(request, dict(turn=dict(id='turn')))
            send(dict(jsonrpc='2.0', id='permission', method='item/commandExecution/requestApproval', params=dict(command='fixture action', cwd=str(work))))
        else:
            send(dict(jsonrpc='2.0', id='permission', method='session/request_permission', params=dict(
                sessionId='session', toolCall=dict(title='fixture action'), options=[
                    dict(kind='allow_always', optionId='always'),
                    dict(kind='allow_once', optionId='once'),
                    dict(kind='reject_once', optionId='deny')])) )
    elif request.get('id') == 'permission':
        approved = request['result'].get('decision') == 'accept' if protocol == 'codex' else request['result']['outcome'].get('optionId') == 'once'
        if approved:
            work.joinpath('effect').write_text('approved once')
        text = 'مرحبا Orbit' if approved else 'Denied safely'
        if protocol == 'codex':
            send(dict(method='turn/completed', params=dict(threadId='child', turn=dict(status='completed'))))
            send(dict(method='item/agentMessage/delta', params=dict(delta=text)))
            send(dict(method='turn/completed', params=dict(threadId='thread', turn=dict(status='completed'))))
        else:
            send(dict(method='session/update', params=dict(sessionId='session', update=dict(
                sessionUpdate='agent_message_chunk', content=dict(type='text', text=text)))))
            reply(pending, dict(stopReason='end_turn'))
