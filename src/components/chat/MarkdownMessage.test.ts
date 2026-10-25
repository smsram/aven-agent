import test from 'node:test';
import assert from 'node:assert/strict';
import { preprocessStreamingMarkdown, isSafeUrl, sanitizeAssistantVisibleText } from './markdownUtils.ts';

test('isSafeUrl allows secure and web protocols', () => {
  assert.equal(isSafeUrl('https://example.com'), true);
  assert.equal(isSafeUrl('https://cloudflare.com/workers-ai/api'), true);
  assert.equal(isSafeUrl('http://localhost:3000'), true);
  assert.equal(isSafeUrl('mailto:support@aven.ai'), true);
});

test('isSafeUrl rejects unsafe or malicious schemes', () => {
  assert.equal(isSafeUrl('javascript:alert(document.cookie)'), false);
  assert.equal(isSafeUrl('data:text/html,<script>alert(1)</script>'), false);
  assert.equal(isSafeUrl('file:///C:/Windows/System32/cmd.exe'), false);
  assert.equal(isSafeUrl('vbscript:MsgBox("hack")'), false);
  assert.equal(isSafeUrl(''), false);
  assert.equal(isSafeUrl(undefined), false);
  assert.equal(isSafeUrl('not a valid url at all'), false);
});

test('preprocessStreamingMarkdown removes internal reasoning tags', () => {
  const input = '<thought>I will inspect the active window to locate the button</thought>Here is the plan for opening Word.';
  const cleaned = preprocessStreamingMarkdown(input, false);
  assert.equal(cleaned.trim(), 'Here is the plan for opening Word.');

  const inputThinking = '<thinking>\nPlanning steps 1 to 3\n</thinking>Executing your request.';
  const cleanedThinking = preprocessStreamingMarkdown(inputThinking, false);
  assert.equal(cleanedThinking.trim(), 'Executing your request.');

  // Open thinking tag during active streaming should be hidden
  const streamingOpen = 'I am working on this. <thought>Analyzing screen elements...';
  const cleanedStreaming = preprocessStreamingMarkdown(streamingOpen, true);
  assert.equal(cleanedStreaming.trim(), 'I am working on this.');
});

test('preprocessStreamingMarkdown removes tool call tags and tool call JSON blocks', () => {
  const toolTagInput = 'I will now click the button. <tool_call>{"name": "desktop_control_mouse_action", "arguments": {"x": 100, "y": 200}}</tool_call>';
  const cleanedTool = preprocessStreamingMarkdown(toolTagInput, false);
  assert.equal(cleanedTool.trim(), 'I will now click the button.');

  const jsonFenceInput = 'Sure! ```json\n{\n  "name": "desktop_control_launch_app",\n  "arguments": {"app_name": "notepad"}\n}\n```Done!';
  const cleanedFence = preprocessStreamingMarkdown(jsonFenceInput, false);
  assert.equal(cleanedFence.trim(), 'Sure! Done!');
});

test('preprocessStreamingMarkdown balances unclosed streaming markdown syntax', () => {
  // Incomplete URL ending with path
  const unclosedUrl = 'Click here: [Aven Docs](https://aven.ai/docs/quickstart';
  const balancedUrl = preprocessStreamingMarkdown(unclosedUrl, true);
  assert.equal(balancedUrl, 'Click here: [Aven Docs](https://aven.ai/docs/quickstart)');

  // Incomplete URL ending with paren
  const unstartedUrl = 'Read more at [Documentation](';
  const balancedUnstarted = preprocessStreamingMarkdown(unstartedUrl, true);
  assert.equal(balancedUnstarted, 'Read more at Documentation');

  // Incomplete trailing backtick for inline code
  const unclosedCode = 'Run the command `npm run dev';
  const balancedCode = preprocessStreamingMarkdown(unclosedCode, true);
  assert.equal(balancedCode, 'Run the command `npm run dev`');

  // Incomplete bold markers
  const unclosedBold = 'This is **very important';
  const balancedBold = preprocessStreamingMarkdown(unclosedBold, true);
  assert.equal(balancedBold, 'This is **very important**');

  // Incomplete table row missing trailing pipe
  const unclosedTableRow = '| Feature | Status |\n| Desktop Control | Active';
  const balancedTableRow = preprocessStreamingMarkdown(unclosedTableRow, true);
  assert.equal(balancedTableRow, '| Feature | Status |\n| Desktop Control | Active |');
});

test('sanitizeAssistantVisibleText scrubs reasoning and tool calls while preserving formatting', () => {
  const raw = `### Task Summary

Here are the actions completed:
- Opened Notepad
- Typed greeting
<thought>Verification passed with 100% confidence.</thought>
\`\`\`bash
echo "All steps succeeded"
\`\`\`
<tool_call>{"name": "desktop_control_capture_screenshot"}</tool_call>`;

  const sanitized = sanitizeAssistantVisibleText(raw);
  assert.ok(!sanitized.includes('<thought>'));
  assert.ok(!sanitized.includes('Verification passed with 100% confidence'));
  assert.ok(!sanitized.includes('<tool_call>'));
  assert.ok(sanitized.includes('### Task Summary'));
  assert.ok(sanitized.includes('- Opened Notepad'));
  assert.ok(sanitized.includes('- Typed greeting'));
  assert.ok(sanitized.includes('echo "All steps succeeded"'));
});
