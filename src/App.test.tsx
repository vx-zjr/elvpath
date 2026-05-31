import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { App } from "./App";

describe("ElvPath app shell", () => {
  test("renders categorized catalog and searchable seed tools", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "ElvPath" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Languages/i })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Search catalog"), { target: { value: "rust" } });

    expect(screen.getByText("Rust")).toBeInTheDocument();
    expect(screen.queryByText("Docker")).not.toBeInTheDocument();
  });

  test("shows install plan with URL permissions checksum and config changes", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /Add Rust/i }));

    expect(screen.getByRole("heading", { name: /Install Plan/i })).toBeInTheDocument();
    expect(screen.getByText(/official source/i)).toBeInTheDocument();
    expect(screen.getByText(/requires admin/i)).toBeInTheDocument();
    expect(screen.getByText(/checksum/i)).toBeInTheDocument();
    expect(screen.getByText("Config changes: PATH")).toBeInTheDocument();
  });

  test("starts plan execution feedback when play is clicked", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /Add Rust/i }));
    fireEvent.click(screen.getByRole("button", { name: /Start installation/i }));

    expect(await screen.findByText(/Browser preview cannot install/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Install terminal/i)).toHaveTextContent(/Dry run:/i);
  });

  test("filters catalog with frontend and backend tabs", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("tab", { name: /Frontend/i }));
    expect(screen.getByText("Node.js")).toBeInTheDocument();
    expect(screen.queryByText("PostgreSQL")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /Backend/i }));
    fireEvent.click(screen.getByRole("button", { name: /Databases/i }));
    expect(screen.getByText("PostgreSQL")).toBeInTheDocument();
  });

  test("accepts an http proxy and includes it in dry-run terminal output", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /Proxy/i }));
    fireEvent.change(screen.getByLabelText(/HTTP\(S\) proxy/i), {
      target: { value: "http://127.0.0.1:7890" }
    });
    fireEvent.click(screen.getByRole("button", { name: /Add Rust/i }));
    fireEvent.click(screen.getByRole("button", { name: /Start installation/i }));

    expect(await screen.findByLabelText(/Install terminal/i)).toHaveTextContent("http://127.0.0.1:7890");
  });

  test("switches between English and Simplified Chinese copy", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "中文" }));

    expect(screen.getByRole("heading", { name: "ElvPath" })).toBeInTheDocument();
    expect(screen.getByText("安装计划")).toBeInTheDocument();
    expect(screen.getByLabelText("搜索目录")).toBeInTheDocument();
  });
});
