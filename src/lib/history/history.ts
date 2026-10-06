export class History<T> {
  private past: T[] = [];
  private future: T[] = [];
  constructor(private limit = 100) {}
  get canUndo() {
    return this.past.length > 0;
  }
  get canRedo() {
    return this.future.length > 0;
  }
  push(current: T) {
    this.past.push(current);
    if (this.past.length > this.limit) this.past.shift();
    this.future = [];
  }
  undo(current: T): T | undefined {
    const value = this.past.pop();
    if (value !== undefined) this.future.push(current);
    return value;
  }
  redo(current: T): T | undefined {
    const value = this.future.pop();
    if (value !== undefined) this.past.push(current);
    return value;
  }
  clear() {
    this.past = [];
    this.future = [];
  }
}
