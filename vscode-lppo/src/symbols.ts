import * as vscode from 'vscode';

export interface LppoSymbol {
  name: string;
  kind: vscode.SymbolKind;
  range: vscode.Range;
  selectionRange: vscode.Range;
  containerName?: string;
  detail?: string;
  children?: LppoSymbol[];
}

const DECL_RE = /^\s*(?:pub\s+)?(?:unsafe\s+)?(?:async\s+)?\b(class|struct|enum|trait|module|proc)\b/;
const IMPL_RE = /^\s*(?:pub\s+)?\bimpl\b\s+(?:([A-Za-z_]\w*)\s+\bfor\b\s+)?([A-Za-z_]\w*)/;
const TYPE_ALIAS_RE = /^\s*(?:pub\s+)?\btype\s+([A-Za-z_]\w*)\s*=/;
const PROC_RE = /^\s*(?:pub\s+)?(?:unsafe\s+)?(?:async\s+)?\bproc\s+([A-Za-z_]\w*)/;
const FIELD_RE = /^\s*(?:pub\s+)?([A-Za-z_]\w*)\s*:\s*([^;,]+)[;,]?\s*$/;
const VARIANT_RE = /^\s*([A-Za-z_]\w*)\s*(\([^)]*\))?[;,]?\s*$/;
const NAME_RE = /[A-Za-z_]\w*/;

const KIND_LABEL: Record<vscode.SymbolKind, string> = {
  [vscode.SymbolKind.File]: 'File',
  [vscode.SymbolKind.Module]: 'Module',
  [vscode.SymbolKind.Namespace]: 'Namespace',
  [vscode.SymbolKind.Package]: 'Package',
  [vscode.SymbolKind.Class]: 'Class',
  [vscode.SymbolKind.Method]: 'Method',
  [vscode.SymbolKind.Property]: 'Property',
  [vscode.SymbolKind.Field]: 'Field',
  [vscode.SymbolKind.Constructor]: 'Constructor',
  [vscode.SymbolKind.Enum]: 'Enum',
  [vscode.SymbolKind.Interface]: 'Interface',
  [vscode.SymbolKind.Function]: 'Function',
  [vscode.SymbolKind.Variable]: 'Variable',
  [vscode.SymbolKind.Constant]: 'Constant',
  [vscode.SymbolKind.String]: 'String',
  [vscode.SymbolKind.Number]: 'Number',
  [vscode.SymbolKind.Boolean]: 'Boolean',
  [vscode.SymbolKind.Array]: 'Array',
  [vscode.SymbolKind.Object]: 'Object',
  [vscode.SymbolKind.Key]: 'Key',
  [vscode.SymbolKind.Null]: 'Null',
  [vscode.SymbolKind.EnumMember]: 'EnumMember',
  [vscode.SymbolKind.Struct]: 'Struct',
  [vscode.SymbolKind.Event]: 'Event',
  [vscode.SymbolKind.Operator]: 'Operator',
  [vscode.SymbolKind.TypeParameter]: 'TypeParameter',
};

export function kindLabel(kind: vscode.SymbolKind): string {
  return KIND_LABEL[kind] ?? 'Symbol';
}

function leadingIndent(line: string): number {
  let n = 0;
  while (n < line.length && (line[n] === ' ' || line[n] === '\t')) {
    n++;
  }
  return n;
}

function stripComment(line: string): string {
  const idx = line.indexOf('//');
  return idx === -1 ? line : line.slice(0, idx);
}

function nameCharOffset(line: string, name: string): number {
  return line.indexOf(name);
}

function findCloseBrace(lines: string[], openLine: number): number {
  let depth = 0;
  let started = false;
  for (let i = openLine; i < lines.length; i++) {
    const line = stripComment(lines[i]);
    for (let i2 = 0; i2 < line.length; i2++) {
      const ch = line[i2];
      if (ch === '{') {
        depth++;
        started = true;
      } else if (ch === '}') {
        depth--;
        if (started && depth === 0) {
          return i;
        }
      }
    }
  }
  return lines.length - 1;
}

function symbolAt(
  name: string,
  kind: vscode.SymbolKind,
  line: number,
  charStart: number,
  charEnd: number,
  endLine: number,
  detail?: string,
  containerName?: string,
  children?: LppoSymbol[],
): LppoSymbol {
  return {
    name,
    kind,
    range: new vscode.Range(line, 0, endLine, 0),
    selectionRange: new vscode.Range(line, charStart, line, charEnd),
    detail,
    containerName,
    children,
  };
}

function procDetail(stripped: string, name: string): string {
  const m = /^(?:pub\s+)?(?:unsafe\s+)?(?:async\s+)?\bproc\s+[A-Za-z_]\w*\s*\(([^)]*)\)(?:\s*->\s*([^:{]+))?/.exec(
    stripped,
  );
  const params = (m && m[1] ? m[1].trim() : '') ?? '';
  const ret = m && m[2] ? ` -> ${m[2].trim()}` : '';
  return `proc ${name}(${params})${ret}`;
}

function parseMembers(
  lines: string[],
  start: number,
  end: number,
  baseIndent: number,
  container: LppoSymbol,
): LppoSymbol[] {
  const members: LppoSymbol[] = [];
  let j = start;
  while (j < end) {
    const line = lines[j];
    const stripped = stripComment(line).replace(/\s+$/, '');
    const indent = leadingIndent(line);
    if (indent <= baseIndent || stripped === '') {
      j++;
      continue;
    }

    const implM2 = IMPL_RE.exec(stripped);
    if (implM2) {
      const traitName = implM2[1];
      const typeName = implM2[2];
      const name = traitName ? `${traitName} for ${typeName}` : typeName;
      let endLine = j;
      if (stripped.endsWith('{')) {
        endLine = findCloseBrace(lines, j);
      }
      const children = parseMembers(lines, j + 1, endLine, indent, symbolAt(name, vscode.SymbolKind.Class, j, 0, 0, endLine));
      members.push(
        symbolAt(
          name,
          vscode.SymbolKind.Class,
          j,
          implM2.index,
          implM2.index + implM2[0].length,
          endLine,
          `impl ${name}`,
          container.name,
          children,
        ),
      );
      j = endLine + 1;
      continue;
    }

    const declM = DECL_RE.exec(stripped);
    if (declM) {
      const keyword = declM[1];
      const nameM = NAME_RE.exec(
        stripped.slice(declM.index + declM[0].length),
      );
      const name = nameM ? nameM[0] : 'unknown';
      const nameStart =
        declM.index +
        declM[0].length +
        (nameM ? nameM.index : 0);
      let endLine = j;
      if (stripped.endsWith('{')) {
        endLine = findCloseBrace(lines, j);
      }
      const children = parseMembers(
        lines,
        j + 1,
        endLine,
        indent,
        symbolAt(
          name,
          keyword === 'class' ? vscode.SymbolKind.Class :
          keyword === 'struct' ? vscode.SymbolKind.Struct :
          keyword === 'enum' ? vscode.SymbolKind.Enum :
          keyword === 'trait' ? vscode.SymbolKind.Interface :
          vscode.SymbolKind.Module,
          j,
          nameStart,
          nameStart + name.length,
          endLine,
        ),
      );
      members.push(
        symbolAt(
          name,
          keyword === 'class' ? vscode.SymbolKind.Class :
          keyword === 'struct' ? vscode.SymbolKind.Struct :
          keyword === 'enum' ? vscode.SymbolKind.Enum :
          keyword === 'trait' ? vscode.SymbolKind.Interface :
          vscode.SymbolKind.Module,
          j,
          nameStart,
          nameStart + name.length,
          endLine,
          undefined,
          container.name,
          children,
        ),
      );
      j = endLine + 1;
      continue;
    }

    const procM = PROC_RE.exec(stripped);
    if (procM) {
      const name = procM[1];
      const nameStart = procM.index + procM[0].lastIndexOf(name);
      let endLine = j;
      if (stripped.endsWith('{')) {
        endLine = findCloseBrace(lines, j);
      }
      members.push(
        symbolAt(
          name,
          vscode.SymbolKind.Method,
          j,
          nameStart,
          nameStart + name.length,
          endLine,
          procDetail(stripped, name),
          container.name,
        ),
      );
      j = endLine + 1;
      continue;
    }

    const fieldM = FIELD_RE.exec(stripped);
    if (fieldM) {
      const name = fieldM[1];
      const nameStart = nameCharOffset(stripped, name);
      members.push(
        symbolAt(
          name,
          vscode.SymbolKind.Field,
          j,
          leadingIndent(line) + nameStart,
          leadingIndent(line) + nameStart + name.length,
          j,
          `${name}: ${fieldM[2].trim()}`,
          container.name,
        ),
      );
      j++;
      continue;
    }

    if (container.kind === vscode.SymbolKind.Enum) {
      const variantM = VARIANT_RE.exec(stripped);
      if (variantM && !stripped.includes(':')) {
        const name = variantM[1];
        const nameStart = variantM.index;
        members.push(
          symbolAt(
            name,
            vscode.SymbolKind.EnumMember,
            j,
            leadingIndent(line) + nameStart,
            leadingIndent(line) + nameStart + name.length,
            j,
            'enum variant',
            container.name,
          ),
        );
        j++;
        continue;
      }
    }

    j++;
  }
  return members;
}

export function parseSymbols(text: string): LppoSymbol[] {
  const lines = text.split('\n');
  const symbols: LppoSymbol[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    const stripped = stripComment(line).replace(/\s+$/, '');
    const indent = leadingIndent(line);
    if (indent !== 0 || stripped === '') {
      i++;
      continue;
    }

    const implM = IMPL_RE.exec(stripped);
    if (implM) {
      const traitName = implM[1];
      const typeName = implM[2];
      const name = traitName ? `${traitName} for ${typeName}` : typeName;
      const typeNameStart = stripped.indexOf(typeName, implM.index + implM[0].lastIndexOf(traitName || 'impl'));
      let endLine = i;
      if (stripped.endsWith('{')) {
        endLine = findCloseBrace(lines, i);
      }
      const children = parseMembers(lines, i + 1, endLine, 0, symbolAt(name, vscode.SymbolKind.Class, i, 0, 0, endLine));
      symbols.push(
        symbolAt(
          name,
          vscode.SymbolKind.Class,
          i,
          implM.index,
          implM.index + implM[0].length,
          endLine,
          `impl ${name}`,
          undefined,
          children,
        ),
      );
      i = endLine + 1;
      continue;
    }

    const typeAliasM = TYPE_ALIAS_RE.exec(stripped);
    if (typeAliasM) {
      const name = typeAliasM[1];
      const nameStart = stripped.indexOf(name);
      symbols.push(
        symbolAt(
          name,
          vscode.SymbolKind.Class,
          i,
          leadingIndent(line) + nameStart,
          leadingIndent(line) + nameStart + name.length,
          i,
          `type alias = ${stripped.slice(stripped.indexOf('=') + 1).trim()}`,
        ),
      );
      i++;
      continue;
    }

    const declM = DECL_RE.exec(stripped);
    if (declM) {
      const keyword = declM[1];
      const rest = stripped.slice(declM.index + declM[0].length);
      const nameM = NAME_RE.exec(rest);
      if (!nameM) {
        i++;
        continue;
      }
      const name = nameM[0];
      const nameStart = declM.index + declM[0].length + nameM.index;
      let endLine = i;
      if (stripped.endsWith('{')) {
        endLine = findCloseBrace(lines, i);
      }
      const containerSym = symbolAt(
        name,
        keyword === 'class' ? vscode.SymbolKind.Class :
        keyword === 'struct' ? vscode.SymbolKind.Struct :
        keyword === 'enum' ? vscode.SymbolKind.Enum :
        keyword === 'trait' ? vscode.SymbolKind.Interface :
        vscode.SymbolKind.Module,
        i,
        leadingIndent(line) + nameStart,
        leadingIndent(line) + nameStart + name.length,
        endLine,
      );
      const children = keyword === 'proc'
        ? []
        : parseMembers(lines, i + 1, endLine, 0, containerSym);
      let detail: string | undefined;
      if (keyword === 'proc') {
        detail = procDetail(stripped, name);
      }
      symbols.push(
        symbolAt(
          name,
          keyword === 'class' ? vscode.SymbolKind.Class :
          keyword === 'struct' ? vscode.SymbolKind.Struct :
          keyword === 'enum' ? vscode.SymbolKind.Enum :
          keyword === 'trait' ? vscode.SymbolKind.Interface :
          vscode.SymbolKind.Function,
          i,
          leadingIndent(line) + nameStart,
          leadingIndent(line) + nameStart + name.length,
          endLine,
          detail,
          undefined,
          children,
        ),
      );
      i = endLine + 1;
      continue;
    }

    i++;
  }
  return symbols;
}

export function flattenSymbols(symbols: LppoSymbol[]): LppoSymbol[] {
  const out: LppoSymbol[] = [];
  for (const s of symbols) {
    out.push(s);
    if (s.children) {
      out.push(...flattenSymbols(s.children));
    }
  }
  return out;
}

export function rangeContains(range: vscode.Range, pos: vscode.Position): boolean {
  return pos.isAfterOrEqual(range.start) && pos.isBeforeOrEqual(range.end);
}

export function getWordAt(document: vscode.TextDocument, position: vscode.Position): string | null {
  const range = document.getWordRangeAtPosition(position);
  if (!range) {
    return null;
  }
  return document.getText(range);
}

export function toDocumentSymbol(sym: LppoSymbol): vscode.DocumentSymbol {
  const ds = new vscode.DocumentSymbol(
    sym.name,
    sym.detail ?? '',
    sym.kind,
    sym.range,
    sym.selectionRange,
  );
  ds.children = (sym.children ?? []).map(toDocumentSymbol);
  return ds;
}

export function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
