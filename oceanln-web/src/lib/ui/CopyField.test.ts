import { render, screen, fireEvent } from "@testing-library/svelte";
import { describe, it, expect } from "vitest";
import CopyField from "./CopyField.svelte";

describe("CopyField", () => {
  it("renders the label, chip and value", () => {
    render(CopyField, { props: { label: "Payout address", chip: "bc1q", value: "bc1qABC123" } });
    expect(screen.getByText("Payout address")).toBeInTheDocument();
    expect(screen.getByText("bc1q")).toBeInTheDocument();
    expect(screen.getByText("bc1qABC123")).toBeInTheDocument();
  });

  it("flips to 'Copied' when the copy button is clicked", async () => {
    render(CopyField, { props: { label: "Signature", value: "sig==" } });
    const btn = screen.getByRole("button", { name: /copy/i });
    await fireEvent.click(btn);
    expect(screen.getByRole("button", { name: /copied/i })).toBeInTheDocument();
  });
});
