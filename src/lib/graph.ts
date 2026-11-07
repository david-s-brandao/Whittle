/**
 * Commit-graph lane assignment.
 *
 * Input: commits newest-first, children always before parents (what `git log --topo-order` gives).
 * Output: one `GraphRow` per commit saying which lane the commit sits in and which line
 * segments cross its row. The renderer only has to draw what it is told.
 *
 * Rules (what makes the graph stable and compact):
 *  - A lane "waits" for one commit id: the next ancestor that lane's line will reach.
 *  - A commit takes the leftmost lane waiting for it; a branch tip nobody waits for takes the
 *    leftmost free lane, so lanes are reused as soon as a line ends.
 *  - The first parent continues in the commit's own lane (the main line stays straight);
 *    further parents (merges) join an existing lane that already waits for them, or open a new one.
 *  - A lane keeps its colour from the moment it opens until it ends.
 */

export interface GraphCommit {
  id: string
  parents: string[]
}

/**
 * Line segment within one row, with the row's vertical span as 0 (top) .. 1 (bottom) and the
 * commit node at 0.5:
 *  - `pass`: lane `from` goes straight through the whole row.
 *  - `in`:   from lane `from` at the top down to the node (in lane `to`).
 *  - `out`:  from the node (lane `from`) down to lane `to` at the bottom.
 */
export interface Edge {
  kind: 'pass' | 'in' | 'out'
  from: number
  to: number
  /** Index into the lane colour palette. */
  color: number
}

export interface GraphRow {
  lane: number
  color: number
  merge: boolean
  edges: Edge[]
}

export class GraphBuilder {
  /** lane -> id of the commit this lane's line is heading to (null = free). */
  #lanes: (string | null)[] = []
  #colors: number[] = []
  #nextColor = 0
  /** Highest number of simultaneously used lanes seen so far (for sizing the graph column). */
  maxLanes = 1

  #firstFree(): number {
    const i = this.#lanes.indexOf(null)
    return i === -1 ? this.#lanes.length : i
  }

  #open(lane: number, target: string, color: number) {
    this.#lanes[lane] = target
    this.#colors[lane] = color
  }

  push(c: GraphCommit): GraphRow {
    const lanes = this.#lanes
    const incoming: number[] = []
    lanes.forEach((target, i) => {
      if (target === c.id) incoming.push(i)
    })

    let lane: number
    let color: number
    if (incoming.length) {
      lane = incoming[0]
      color = this.#colors[lane]
    } else {
      lane = this.#firstFree()
      color = this.#nextColor++
    }

    const edges: Edge[] = []
    lanes.forEach((target, i) => {
      if (target !== null && target !== c.id) {
        edges.push({ kind: 'pass', from: i, to: i, color: this.#colors[i] })
      }
    })
    for (const i of incoming) {
      edges.push({ kind: 'in', from: i, to: lane, color: this.#colors[i] })
      lanes[i] = null
    }

    if (c.parents.length) {
      this.#open(lane, c.parents[0], color)
      edges.push({ kind: 'out', from: lane, to: lane, color })
      for (const p of c.parents.slice(1)) {
        let d = lanes.indexOf(p)
        if (d === -1) {
          d = this.#firstFree()
          this.#open(d, p, this.#nextColor++)
        }
        edges.push({ kind: 'out', from: lane, to: d, color: this.#colors[d] })
      }
    }

    this.maxLanes = Math.max(this.maxLanes, lanes.length)
    while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop()
    return { lane, color, merge: c.parents.length > 1, edges }
  }
}

export function buildGraph(commits: GraphCommit[]): { rows: GraphRow[]; maxLanes: number } {
  const b = new GraphBuilder()
  const rows = commits.map((c) => b.push(c))
  return { rows, maxLanes: b.maxLanes }
}
