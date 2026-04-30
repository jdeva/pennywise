/**
 * Tree parser utilities for converting flat colon-separated account names
 * into a hierarchical tree structure and back.
 */

export type AccountType = 'assets' | 'expenses' | 'income' | 'liabilities' | 'equity';

export interface AccountTreeNode {
  /** Segment name, e.g., "Food" */
  name: string;
  /** Full colon-separated path from root, e.g., "Expenses:Food" */
  fullPath: string;
  /** Child nodes; empty array for leaves */
  children: AccountTreeNode[];
  /** Whether this node was explicitly in the input (vs. created as an intermediate) */
  isExplicit: boolean;
}

export interface AccountTypeTree {
  type: AccountType;
  /** Capitalized display label, e.g., "Expenses" */
  label: string;
  children: AccountTreeNode[];
}

/**
 * Parse a flat list of colon-separated account names into a tree (trie).
 *
 * 1. Sort the input for deterministic output
 * 2. For each name, split by ':'
 * 3. Walk the tree from root, creating intermediate nodes as needed
 * 4. Mark nodes that correspond to actual input names as explicit
 */
export function buildTree(accountNames: string[]): AccountTreeNode[] {
  const sorted = [...accountNames].sort();
  const roots: AccountTreeNode[] = [];

  for (const name of sorted) {
    const segments = name.split(':');
    let currentLevel = roots;
    let pathSoFar = '';

    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i];
      pathSoFar = i === 0 ? segment : `${pathSoFar}:${segment}`;

      let existing = currentLevel.find((n) => n.name === segment);
      if (!existing) {
        existing = {
          name: segment,
          fullPath: pathSoFar,
          children: [],
          isExplicit: false,
        };
        currentLevel.push(existing);
      }

      // Mark as explicit if this is the full input name
      if (i === segments.length - 1) {
        existing.isExplicit = true;
      }

      currentLevel = existing.children;
    }
  }

  return roots;
}

/**
 * Reconstruct flat account names from a tree by collecting all explicit nodes.
 * Returns a sorted list matching the original input to buildTree.
 */
export function flattenTree(nodes: AccountTreeNode[]): string[] {
  const result: string[] = [];

  function walk(nodeList: AccountTreeNode[]): void {
    for (const node of nodeList) {
      if (node.isExplicit) {
        result.push(node.fullPath);
      }
      walk(node.children);
    }
  }

  walk(nodes);
  return result.sort();
}

/** Display labels for each account type */
const ACCOUNT_TYPE_LABELS: Record<AccountType, string> = {
  assets: 'Assets',
  expenses: 'Expenses',
  income: 'Income',
  liabilities: 'Liabilities',
  equity: 'Equity',
};

/**
 * Build an AccountTypeTree for a given account type and its names.
 * Strips the type prefix from account names if present (e.g., "Assets:Bank" → "Bank")
 * since the root section already displays the type label.
 */
export function buildAccountTypeTree(
  type: AccountType,
  accountNames: string[],
): AccountTypeTree {
  const prefix = ACCOUNT_TYPE_LABELS[type] + ':';
  const stripped = accountNames.map((name) =>
    name.startsWith(prefix) ? name.slice(prefix.length) : name
  );
  return {
    type,
    label: ACCOUNT_TYPE_LABELS[type],
    children: buildTree(stripped),
  };
}
