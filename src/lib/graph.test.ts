import { describe, expect, it } from 'vitest'
import { buildGraph, type GraphCommit } from './graph'

const c = (id: string, ...parents: string[]): GraphCommit => ({ id, parents })
const kinds = (row: { edges: { kind: string }[] }) => row.edges.map((e) => e.kind).sort()

describe('graph lanes', () => {
  it('keeps a linear history in lane 0', () => {
    const { rows, maxLanes } = buildGraph([c('c', 'b'), c('b', 'a'), c('a')])
    expect(rows.map((r) => r.lane)).toEqual([0, 0, 0])
    expect(maxLanes).toBe(1)
    expect(kinds(rows[0])).toEqual(['out']) // tip: only goes down
    expect(kinds(rows[1])).toEqual(['in', 'out'])
    expect(kinds(rows[2])).toEqual(['in']) // root: only comes from above
    expect(new Set(rows.map((r) => r.color)).size).toBe(1)
  })

  it('draws a merge: second parent opens lane 1, both converge on the fork point', () => {
    //  M (merge of A and B)
    //  |\
    //  A B
    //  |/
    //  R
    const { rows } = buildGraph([c('M', 'A', 'B'), c('A', 'R'), c('B', 'R'), c('R')])
    expect(rows.map((r) => r.lane)).toEqual([0, 0, 1, 0])
    expect(rows[0].merge).toBe(true)
    const mergeOut = rows[0].edges.filter((e) => e.kind === 'out')
    expect(mergeOut.map((e) => e.to).sort()).toEqual([0, 1])
    // while B is drawn, lane 0 (waiting for R) passes straight through
    expect(rows[2].edges.some((e) => e.kind === 'pass' && e.from === 0)).toBe(true)
    // R receives both lines
    const into = rows[3].edges.filter((e) => e.kind === 'in')
    expect(into.map((e) => e.from).sort()).toEqual([0, 1])
    expect(into.every((e) => e.to === 0)).toBe(true)
  })

  it('reuses freed lanes instead of growing', () => {
    // a side branch is merged back, then a new unrelated branch tip appears
    const { rows, maxLanes } = buildGraph([
      c('M', 'A', 'B'),
      c('A', 'R'),
      c('B', 'R'),
      c('R', 'Z'),
      c('T', 'Z'), // new tip arrives after lane 1 was freed
      c('Z'),
    ])
    expect(maxLanes).toBe(2)
    expect(rows[4].lane).toBe(1) // T reuses lane 1
  })

  it('keeps colours stable along a lane and distinct between neighbours', () => {
    const { rows } = buildGraph([
      c('M', 'A', 'B'),
      c('A', 'A2'),
      c('B', 'B2'),
      c('A2', 'R'),
      c('B2', 'R'),
      c('R'),
    ])
    const laneColor = (row: number, lane: number) =>
      rows[row].edges.find((e) => e.from === lane)?.color
    // lane 0 and lane 1 differ
    expect(rows[0].color).not.toBe(laneColor(1, 1) ?? rows[0].color)
    // lane 1's colour is identical on every row it appears in
    expect(
      new Set([rows[2].color, rows[4].edges.find((e) => e.kind === 'in' && e.from === 1)?.color]),
    ).toEqual(new Set([rows[2].color]))
  })

  it('handles two branch tips side by side, merging later', () => {
    // t1 and t2 are separate heads sharing ancestor base
    const { rows } = buildGraph([c('t1', 'base'), c('t2', 'base'), c('base')])
    expect(rows.map((r) => r.lane)).toEqual([0, 1, 0])
    expect(kinds(rows[2])).toEqual(['in', 'in'])
  })

  it('an octopus merge opens one lane per extra parent', () => {
    const { rows, maxLanes } = buildGraph([c('O', 'a', 'b', 'c'), c('a'), c('b'), c('c')])
    expect(
      rows[0].edges
        .filter((e) => e.kind === 'out')
        .map((e) => e.to)
        .sort(),
    ).toEqual([0, 1, 2])
    expect(maxLanes).toBe(3)
  })

  it('leaves a lane open when the parent is not loaded yet (paging / shallow clones)', () => {
    const { rows } = buildGraph([c('x', 'missing'), c('y', 'also-missing')])
    expect(rows[1].lane).toBe(1) // x's line still occupies lane 0
    expect(rows[1].edges.some((e) => e.kind === 'pass' && e.from === 0)).toBe(true)
  })

  it('is incremental: building in pages equals building at once', () => {
    const commits: GraphCommit[] = []
    for (let i = 0; i < 500; i++) {
      commits.push(c(`n${i}`, `n${i + 1}`, ...(i % 7 === 0 ? [`n${i + 3}`] : [])))
    }
    const whole = buildGraph(commits).rows
    const paged = buildGraph(commits.slice(0, 200)).rows
    // a fresh builder fed the same sequence in pieces must give identical rows: prefix property
    expect(whole.slice(0, 200)).toEqual(paged)
  })

  it('handles 100k commits quickly', () => {
    const commits: GraphCommit[] = Array.from({ length: 100_000 }, (_, i) =>
      c(`c${i}`, `c${i + 1}`),
    )
    const t = performance.now()
    const { rows } = buildGraph(commits)
    expect(rows.length).toBe(100_000)
    expect(performance.now() - t).toBeLessThan(1500)
  })
})
