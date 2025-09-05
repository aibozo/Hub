export type Segment = { type: 'text' | 'code'; content: string; lang?: string };

export function parseFences(input: string): Segment[] {
  const lines = input.replaceAll('\r\n', '\n').split('\n');
  const segs: Segment[] = [];
  let inCode = false;
  let lang = '';
  let buf: string[] = [];
  const flushText = () => { if (buf.length) { segs.push({ type: 'text', content: buf.join('\n') }); buf = []; } };
  const flushCode = () => { segs.push({ type: 'code', content: buf.join('\n'), lang: lang || undefined }); buf = []; lang=''; };
  for (const line of lines) {
    const fence = line.startsWith('```');
    if (fence) {
      if (!inCode) { flushText(); inCode = true; lang = line.slice(3).trim(); continue; }
      else { inCode = false; flushCode(); continue; }
    }
    buf.push(line);
  }
  if (buf.length) { (inCode ? flushCode : flushText)(); }
  return segs;
}

