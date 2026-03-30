import SwiftUI

struct DebugOverlay: View {
    @State private var isPanelPresented = false

    var body: some View {
        Button(action: {
            isPanelPresented.toggle()
        }) {
            Image(systemName: "ladybug.fill")
                .font(.title2)
                .foregroundColor(.white)
                .frame(width: 50, height: 50)
                .background(Color.red.opacity(0.8))
                .clipShape(Circle())
                .shadow(radius: 4)
        }
        .padding()
        .sheet(isPresented: $isPanelPresented) {
            DebugPanelView()
                .presentationDetents([.medium, .large])
                .presentationDragIndicator(.visible)
        }
    }
}

struct DebugPanelView: View {
    @State private var selectedTab = 0

    var body: some View {
        VStack(spacing: 0) {
            // Tab Bar
            HStack(spacing: 0) {
                TabButton(title: "Hierarchy", isSelected: selectedTab == 0) {
                    selectedTab = 0
                }
                TabButton(title: "Console", isSelected: selectedTab == 1) {
                    selectedTab = 1
                }
                TabButton(title: "State", isSelected: selectedTab == 2) {
                    selectedTab = 2
                }
                TabButton(title: "Network", isSelected: selectedTab == 3) {
                    selectedTab = 3
                }
            }
            .padding(.horizontal)
            .padding(.top, 8)

            Divider()
                .padding(.top, 8)

            // Content
            TabView(selection: $selectedTab) {
                ViewHierarchyView()
                    .tag(0)
                ConsoleLogView()
                    .tag(1)
                StateInspectorView()
                    .tag(2)
                NetworkLogView()
                    .tag(3)
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
        }
        .background(Color(UIColor.systemBackground))
    }
}

struct TabButton: View {
    let title: String
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Text(title)
                    .font(.caption)
                    .fontWeight(isSelected ? .semibold : .regular)
                    .foregroundColor(isSelected ? .blue : .secondary)
                Rectangle()
                    .fill(isSelected ? Color.blue : Color.clear)
                    .frame(height: 2)
            }
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity)
    }
}

#Preview {
    DebugOverlay()
}
