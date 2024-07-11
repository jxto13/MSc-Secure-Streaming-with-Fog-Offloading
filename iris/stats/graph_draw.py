import matplotlib.pyplot as plt
import json

def plot_reports(data):
    # Extracting index and report values
    indices = [entry["index"] for entry in data]
    reports = [entry["reports"] for entry in data]

    # Plotting the line graph
    plt.figure(figsize=(10, 6))
    plt.plot(indices, reports, marker='o', color='skyblue', linestyle='-', linewidth=2, markersize=6)
    plt.xlabel('Index')
    plt.ylabel('Reports')
    plt.title('Missed data packets per Index (1 client)')
    plt.xticks(indices[::100])  # Ensure all indices are shown on the x-axis
    plt.grid(axis='y', linestyle='--', alpha=0.7)

    # Show the plot
    plt.show()

def main():

    # Opening JSON file
    f = open('reports.json')

    # returns JSON object as 
    # a dictionary
    data = json.load(f)

    # Plot the reports
    plot_reports(data)

if __name__ == "__main__":
    main()


# import matplotlib.pyplot as plt
# import json

# def plot_reports(data, tick_interval=100):
#     # Extracting index and report values
#     indices = [entry["index"] for entry in data]
#     reports = [entry["reports"] for entry in data]

#     # Plotting the scatter plot (dots)
#     plt.figure(figsize=(10, 6))
#     plt.scatter(indices, reports, marker='o', color='skyblue', s=50)  # s controls the size of dots
#     plt.xlabel('Matrix Index')
#     plt.ylabel('Missed Data Packets')
#     plt.title('Missed data packets per Index (1 client)')

#     # Set y-axis to display only integer values
#     plt.yticks(range(0, max(reports) + 1))
# ac
#     # Set x-ticks at specified intervals
#     plt.xticks(range(min(indices), max(indices) + 1, tick_interval))

#     plt.grid(axis='both', linestyle='--', alpha=0.7)

#     # Show the plot
#     plt.show()

# def main():
#     # Opening JSON file
#     with open('reports.json') as f:
#         data = json.load(f)

#     # Specify the interval for x-tick labels
#     tick_interval = 100

#     # Plot the reports
#     plot_reports(data, tick_interval)

# if __name__ == "__main__":
#     main()
