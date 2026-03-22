/**
 * A single tab item for the TabList component.
 * The label should already be translated by the parent using t().
 */
export interface TabItem<T extends string = string> {
  id: T;
  label: string;
}
