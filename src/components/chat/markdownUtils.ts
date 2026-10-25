/**
 * Utility functions for markdown preprocessing, link validation, and text sanitization.
 */

/**
 * Checks if a URL has an allowed safe protocol (http, https, mailto).
 * Rejects javascript:, data:, file:, vbscript: and malformed URIs.
 */
export function isSafeUrl(href?: string): boolean {
  if (!href) return false;
  try {
    const url = new URL(href);
    return url.protocol === 'http:' || url.protocol === 'https:' || url.protocol === 'mailto:';
  } catch {
    return false;
  }
}

/**
 * Preprocesses streaming markdown text to:
 * 1. Filter internal tool call blocks, thinking tags, or JSON payloads from assistant messages.
 * 2. Balance incomplete syntax (links, code fences, unclosed backticks, bold markers, tables)
 *    so the parser does not flicker or display raw syntax during token streaming.
 */
export function preprocessStreamingMarkdown(raw: string, isStreaming?: boolean): string {
  if (!raw) return '';

  let text = raw;

  // 1. Strip completed or open internal reasoning tags (<thought> or <thinking>)
  text = text.replace(/<(thought|thinking)>[\s\S]*?<\/\1>/gi, '');
  text = text.replace(/<(thought|thinking)>[\s\S]*$/gi, '');

  // 2. Strip completed or open tool call tags (<tool_call>...</tool_call>)
  text = text.replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, '');
  text = text.replace(/<tool_call>[\s\S]*$/gi, '');

  // 3. Strip fenced json blocks that are tool calls (e.g. contains "name": "desktop_control_...)
  text = text.replace(/```(?:json)?\s*\{[\s\S]*?"name"\s*:\s*"(?:desktop_control_|native_|google_search)[\s\S]*?```/gi, '');
  text = text.replace(/```(?:json)?\s*\{[\s\S]*?"name"\s*:\s*"(?:desktop_control_|native_|google_search)[\s\S]*$/gi, '');

  // 4. Strip partial tag remnants at the very end during active streaming
  if (isStreaming) {
    text = text.replace(/<(?:tool_?c?a?l?l?|thought?|thinking?)?$/gi, '');
  }

  if (!isStreaming) return text;

  // 5. Incomplete markdown links:
  // 5a. If ends with `[Label](https://domain.com/path` (missing `)`)
  const unclosedLinkMatch = /\[([^\]]+)\]\((https?:\/\/[^\s\)]*)$/.exec(text);
  if (unclosedLinkMatch) {
    text = `${text})`;
  } else {
    // 5b. If ends with `[Label](` without any URL yet
    const unstartedUrlMatch = /\[([^\]]+)\]\($/.exec(text);
    if (unstartedUrlMatch) {
      text = text.slice(0, unstartedUrlMatch.index) + unstartedUrlMatch[1];
    } else if (text.endsWith('[')) {
      // 5c. Dangling `[`
      text = text.slice(0, -1);
    }
  }

  // 6. Incomplete table row (last line has `| Col 1 | Col 2` without final `|`)
  const lastLineBreak = text.lastIndexOf('\n');
  const lastLine = lastLineBreak >= 0 ? text.slice(lastLineBreak + 1) : text;
  if (lastLine.trim().startsWith('|') && lastLine.includes('|') && !lastLine.trim().endsWith('|')) {
    text = `${text} |`;
  }

  // 7. Incomplete inline code (odd number of single backticks outside fenced blocks)
  const segmentsWithoutFences = text.split(/```[\s\S]*?```/);
  const trailingSegment = segmentsWithoutFences[segmentsWithoutFences.length - 1];
  const backtickMatches = trailingSegment.match(/`/g);
  if (backtickMatches && backtickMatches.length % 2 !== 0 && !trailingSegment.endsWith('```')) {
    text = `${text}\``;
  }

  // 8. Incomplete bold markers: if an odd count of `**` exists in trailing segment
  const boldMatches = trailingSegment.match(/\*\*/g);
  if (boldMatches && boldMatches.length % 2 !== 0) {
    text = `${text}**`;
  }

  return text;
}

/**
 * Sanitizes assistant visible text before saving to chat message state.
 * Strips internal thinking, tool call wrappers, and serialized JSON tool calls.
 */
export function sanitizeAssistantVisibleText(raw: string): string {
  if (!raw) return '';
  let text = raw;

  // 1. Strip completed or open internal reasoning tags (<thought> or <thinking>)
  text = text.replace(/<(thought|thinking)>[\s\S]*?<\/\1>/gi, '');
  text = text.replace(/<(thought|thinking)>[\s\S]*$/gi, '');

  // 2. Strip completed or open tool call tags (<tool_call>...</tool_call>)
  text = text.replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, '');
  text = text.replace(/<tool_call>[\s\S]*$/gi, '');

  // 3. Strip fenced json blocks that are tool calls (e.g. contains "name": "desktop_control_...)
  text = text.replace(/```(?:json)?\s*\{[\s\S]*?"name"\s*:\s*"(?:desktop_control_|native_|google_search)[\s\S]*?```/gi, '');
  text = text.replace(/```(?:json)?\s*\{[\s\S]*?"name"\s*:\s*"(?:desktop_control_|native_|google_search)[\s\S]*$/gi, '');

  return text;
}
