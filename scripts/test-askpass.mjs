import assert from 'node:assert/strict';
import { createServer } from 'node:net';
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';

// Local protocol smoke test with a fabricated response, never real credentials.
const executable = process.argv[2];
if (!executable) throw new Error('Pass the built Git Toys executable path.');
for (const response of ['fabricated-test-response', null]) {
  const token = randomUUID();
  const server = createServer((socket) => {
    let raw = '';
    socket.on('data', (chunk) => {
      raw += chunk;
      if (!raw.includes('\n')) return;
      const request = JSON.parse(raw);
      assert.equal(request.token, token);
      assert.equal(request.prompt, 'Test prompt');
      socket.end(`${JSON.stringify(response)}\n`);
    });
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  try {
    const child = spawn(executable, ['Test prompt'], {
      env: { ...process.env, GIT_TOYS_AUTH_ADDRESS: `127.0.0.1:${server.address().port}`, GIT_TOYS_AUTH_TOKEN: token },
      timeout: 5000,
    });
    let output = '';
    child.stdout.on('data', (chunk) => { output += chunk; });
    const code = await new Promise((resolve, reject) => {
      child.on('error', reject);
      child.on('exit', resolve);
    });
    assert.equal(code, response === null ? 1 : 0);
    assert.equal(output, response === null ? '' : `${response}\n`);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}
console.log('Askpass helper response and cancellation passed (fabricated credentials).');
